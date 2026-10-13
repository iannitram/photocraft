//! Ours (self-logging, never offered upstream; see `crates/craft-log`): the desktop app's logger
//! through craft-log, the diagnostic report at the top of every log, and the lines that say what
//! the app found when its window opened.
//!
//! Upstream's [`logging::AppLogger`] stays the logger that decides and writes (same file, folder,
//! rotation and format); craft-log wraps it to add the build line, `CRAFT_LOG`, the repeat
//! collapse, the ring of recent lines and the stall watch.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use craft_log::report::{Report, last_line_with, mask_home, newest_file};
use photocraft_ui_egui::self_logging::{APP, add_window_fact};

use crate::logging::{self, AppLogger, Filter};

/// The window's inner size `native_options` in main.rs asks for, in points (keep the two equal).
pub const REQUESTED_INNER_SIZE: [f32; 2] = [1440.0, 900.0];
/// The frame at which the window's size is compared with the size asked for: by then the
/// compositor has configured the window and `work_area::fit_window` has had its say.
const SIZE_CHECK_FRAME: u64 = 30;
/// The window app's log after a start rotated it (upstream's name), and the mark our last line
/// carries when a run ends in order.
const PREVIOUS_LOG_FILE: &str = "photocraft.1.log";
const END_MARK: &str = "run ended:";
/// Environment switches PhotoCraft reads besides craft-log's own.
const SWITCH_PREFIXES: &[&str] = &["PHOTOCRAFT_", "WGPU_"];

static TURNED: Mutex<Vec<[u32; 2]>> = Mutex::new(Vec::new());

/// In place of [`logging::install`]: the same logger, wrapped. `None` when another logger was
/// installed first.
pub fn install() -> Option<&'static AppLogger> {
    static INNER: OnceLock<AppLogger> = OnceLock::new();
    let (spec, _) = APP.filter_spec();
    let filter = Filter::parse(&spec);
    let max = filter.max();
    // Upstream's own installer stays in their file, unused while this one is in place; naming it
    // here keeps it compiled and warning-free without editing their file.
    let _upstream: fn() -> Option<&'static AppLogger> = logging::install;
    let inner = INNER.get_or_init(|| AppLogger::new(filter, true));
    craft_log::install_wrapped(&APP, inner, max).then_some(inner)
}

/// The log directory is known: record the log file's path for the lines that point at it.
pub fn log_dir(dir: &Path) {
    craft_log::set_log_path(Some(dir.join(logging::LOG_FILE)));
}

/// What the window is opened on, in words: native Wayland, Xwayland and why, or X11.
#[cfg(target_os = "linux")]
pub fn session_text(native: crate::linux_libs::DisplaySession, window: crate::linux_libs::DisplaySession) -> String {
    use crate::linux_libs::DisplaySession as S;
    match (native, window) {
        (S::Wayland, S::Wayland) => "native Wayland".to_owned(),
        (S::Wayland, S::X11) => {
            let var = |k: &str| std::env::var(k).ok();
            let pen = || std::fs::read_to_string("/proc/bus/input/devices").is_ok_and(|d| crate::tablet::has_pen(&d));
            let libs = || crate::linux_libs::available(S::X11);
            if crate::tablet::pen_needs_xwayland(native, var, pen, libs) {
                "Xwayland (X11 on a Wayland desktop), because a pen was found among the input devices; PHOTOCRAFT_NATIVE_WAYLAND=1 keeps native Wayland".to_owned()
            } else {
                "Xwayland (X11 on a Wayland desktop), because Preferences › Performance › Linux display server is X11".to_owned()
            }
        }
        (S::X11, _) => "X11".to_owned(),
        (_, S::None) | (S::None, _) => "none found (neither WAYLAND_DISPLAY nor DISPLAY is set)".to_owned(),
    }
}

/// [`session_text`] where there is one window system only.
#[cfg(not(target_os = "linux"))]
pub fn session_text_here() -> String {
    format!("the {} window system", std::env::consts::OS)
}

/// Start-up, once the log file and the window's session are settled and before the window
/// opens: write the diagnostic report into the log.
pub fn start_report(settings: Option<&Path>, window_session: &str) {
    let show = |p: Option<&Path>| p.map_or_else(|| craft_log::report::UNKNOWN.to_owned(), |p| mask_home(&p.display().to_string()));
    let mut r = Report::new(&APP);
    r.add("system", "the window is on", window_session);
    r.add("graphics", "adapter", "chosen when the window opens: the \"graphics:\" line after this report");
    r.add("screens", "screens and window", "known when the window opens: the \"screen\" and \"window:\" lines after this report");
    r.add("folders", "settings", show(settings));
    r.add("folders", "preferences", show(settings.map(|d| d.join("preferences.json")).as_deref()));
    r.add_switches(SWITCH_PREFIXES);
    let logs = settings.map(|d| d.join("logs"));
    let previous = logs.as_ref().map(|d| d.join(PREVIOUS_LOG_FILE)).filter(|p| p.is_file());
    r.add(
        "previous run",
        "ended",
        match &previous {
            None => "no earlier log".to_owned(),
            Some(p) => last_line_with(p, END_MARK, 64 * 1024).map_or_else(
                || "without its closing line: it crashed, was killed, or predates this report".to_owned(),
                |line| line.split_once(END_MARK).map_or(line.clone(), |(_, rest)| rest.trim().to_owned()),
            ),
        },
    );
    let file = |name: &str| logs.as_ref().map(|d| d.join(name)).filter(|p| p.is_file());
    let (window, cli) = (file(logging::LOG_FILE), file(&craft_log::cli::file_name(APP.name)));
    let crash = logs.as_ref().and_then(|d| newest_file(d, &|name| name.starts_with("crash")));
    r.add_logs(logs.as_deref(), &[("window log", window.as_deref()), ("previous window log", previous.as_deref()), ("command-line log", cli.as_deref())], crash.as_deref());
    r.log(module_path!());
    add_window_fact(format!("Window session: {window_session}"));
}

/// The Wayland outputs shown turned a quarter (mode sizes in pixels), as the rotation query found
/// them; [`window_ready`] names them with the screens.
pub fn turned_screens(modes: &[[u32; 2]]) {
    *TURNED.lock().unwrap_or_else(PoisonError::into_inner) = modes.to_vec();
}

fn fact(line: String) {
    log::info!("{line}");
    add_window_fact(line);
}

/// The window exists (in eframe's app creator): log the graphics adapter in use, every screen
/// with its rotation where it can be known, and the window's size asked for and got; and once
/// the compositor and the window-fit step have had their say, the size it ended with.
pub fn window_ready(cc: &eframe::CreationContext<'_>, on_wayland: bool) {
    match &cc.wgpu_render_state {
        Some(rs) => {
            let i = rs.adapter.get_info();
            fact(format!("graphics: the window draws with \"{}\" ({:?}, {:?}, driver {} {})", i.name, i.backend, i.device_type, i.driver, i.driver_info));
        }
        None => log::warn!("decision: graphics: the window has no wgpu device; images are composited on the CPU"),
    }
    let turned = TURNED.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let asked = REQUESTED_INNER_SIZE;
    let Some(w) = cc.winit_window() else {
        fact("screens: unknown (the window system gave no window handle)".to_owned());
        return;
    };
    let current = w.current_monitor();
    let mut screens = 0;
    for (n, m) in w.available_monitors().enumerate() {
        screens += 1;
        let (s, scale) = (m.size(), m.scale_factor());
        let here = if current.as_ref() == Some(&m) { "; the window is on it" } else { "" };
        let rotation = if !on_wayland {
            "rotation already applied by the X server".to_owned()
        } else if turned.contains(&[s.width, s.height]) {
            format!(
                "shown turned a quarter, so the desktop is {} x {} pixels ({:.0} x {:.0} points)",
                s.height,
                s.width,
                f64::from(s.height) / scale,
                f64::from(s.width) / scale
            )
        } else {
            "not turned".to_owned()
        };
        fact(format!(
            "screen {n}: \"{}\" reports {} x {} pixels, scale {scale}; {rotation}; usable area unknown to the app until the window is maximized{here}",
            m.name().unwrap_or_else(|| "unnamed".into()),
            s.width,
            s.height
        ));
    }
    if screens == 0 {
        fact("screens: none reported by the window system".to_owned());
    }
    let (inner, scale) = (w.inner_size(), w.scale_factor());
    let got = inner.to_logical::<f64>(scale);
    fact(format!(
        "window: created at {:.0} x {:.0} points ({} x {} pixels, scale {scale}), asked {} x {}; maximized {}",
        got.width,
        got.height,
        inner.width,
        inner.height,
        asked[0],
        asked[1],
        w.is_maximized()
    ));
    // The size comes from the window itself: egui's `inner_rect` stays empty on Wayland.
    let window = w.clone();
    let done = AtomicBool::new(false);
    cc.egui_ctx.on_end_pass(
        "self-logging-window-size",
        std::sync::Arc::new(move |ui| {
            if ui.ctx().cumulative_frame_nr() < SIZE_CHECK_FRAME || done.swap(true, Ordering::Relaxed) {
                return;
            }
            let (inner, scale) = (window.inner_size(), window.scale_factor());
            let got = inner.to_logical::<f64>(scale);
            let line = format!(
                "window: asked {} x {} points, got {:.0} x {:.0} ({} x {} pixels, scale {scale}); maximized {}, full screen {}",
                asked[0],
                asked[1],
                got.width,
                got.height,
                inner.width,
                inner.height,
                window.is_maximized(),
                window.fullscreen().is_some()
            );
            if (got.width - f64::from(asked[0])).abs() > 1.0 || (got.height - f64::from(asked[1])).abs() > 1.0 {
                log::warn!("decision: {line}");
            } else {
                log::info!("{line}");
            }
            add_window_fact(line);
        }),
    );
}

/// The last line of a run that ends in order; the next start's report looks for it.
pub fn run_ended(in_order: bool) {
    log::info!("{END_MARK} {}", if in_order { "in order" } else { "with an error from the window system" });
    log::logger().flush();
}
