//! The part of the window the user can actually see and reach (#315).
//!
//! A window can be larger than its display: PhotoCraft opens at 1440 × 900 pt, which runs past
//! the bottom of a 1366 × 768 screen and under the Windows taskbar. egui sizes and places menus
//! by the window's content rect, so their last items (and their ▼ scroll arrow) were out of
//! reach. egui doesn't expose the monitor's work area (the screen minus the taskbar or Dock), only
//! the monitor's size and the window's position on the desktop: the visible part is the content
//! rect cut to the monitor, keeping a taskbar-sized margin clear at the monitor's bottom.
//!
//! [`fit_window`] also fixes the cause at startup: a window that doesn't fit its monitor is
//! maximized, which puts it in the work area (Photoshop opens maximized on small displays).

use egui::{Rect, Vec2, pos2, vec2};

/// Room kept free at the monitor's bottom edge for a taskbar or Dock egui can't see (the Windows
/// taskbar is 40–48 pt tall; a bottom Dock about as much).
pub const TASKBAR: f32 = 48.0;
/// Below this, the guess about where the monitor is went wrong: use the whole window.
const MIN_VISIBLE: f32 = 160.0;

/// Where the window's content can be seen, in egui points. `monitor` is the monitor's size,
/// `inner` the content's rect on the desktop (both from [`egui::ViewportInfo`]). `state` says
/// whether the window is maximized or full screen.
pub fn visible_part(content: Rect, monitor: Option<Vec2>, inner: Option<Rect>, state: WindowState) -> Rect {
    let Some(m) = monitor else { return content };
    if state == WindowState::FullScreen || !m.x.is_finite() || !m.y.is_finite() || m.x < 1.0 || m.y < 1.0 {
        return content;
    }
    // A maximized window is usually inside the work area already, but not always (#3737): on X11
    // a window manager that doesn't reserve a panel's space (Cinnamon's Muffin with PhotoCraft's
    // own title bar, a panel without a strut) maximizes it to the whole monitor, and the panel then
    // covers its bottom. Where the window is known, it is checked like any other; only a maximized
    // window with no position (Wayland) is taken to fit.
    if state == WindowState::Maximized && inner.is_none() {
        return content;
    }
    // #3737: Wayland never tells a window where it is (winit's `inner_position` fails), so egui has
    // no `inner_rect` there, only the monitor's size. A window bigger than the monitor less the
    // taskbar still runs under it wherever it is: at best its content starts at the monitor's top,
    // so at most that much of it can be seen.
    let Some(inner) = inner else {
        let room = vec2(m.x, m.y - TASKBAR);
        return trusted(content, Rect::from_min_size(content.min, room));
    };
    if !inner.min.is_finite() || !inner.max.is_finite() {
        return content;
    }
    // Only the monitor's size is known, not where it is on the desktop: assume monitors tile the
    // desktop by their size (true for one monitor, and for side-by-side ones of the same size).
    let c = inner.center();
    let origin = pos2((c.x / m.x).floor() * m.x, (c.y / m.y).floor() * m.y);
    let screen = Rect::from_min_size(origin, m);
    let usable = Rect::from_min_max(screen.min, pos2(screen.max.x, screen.max.y - TASKBAR));
    // Desktop → window coordinates.
    let shift = content.min - inner.min;
    let local = usable.translate(shift);
    trusted(content, local)
}

/// How the window sits on its monitor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Maximized,
    /// Full screen covers the taskbar on purpose: all of it can be seen.
    FullScreen,
}

impl WindowState {
    fn of(v: &egui::ViewportInfo) -> Self {
        if v.fullscreen == Some(true) {
            Self::FullScreen
        } else if v.maximized == Some(true) {
            Self::Maximized
        } else {
            Self::Normal
        }
    }
}

/// `content` cut to `usable`, unless that leaves too little to be a sensible guess.
fn trusted(content: Rect, usable: Rect) -> Rect {
    let visible = content.intersect(usable);
    if !visible.is_positive() || visible.height() < MIN_VISIBLE.min(content.height()) || visible.width() < MIN_VISIBLE.min(content.width()) {
        return content;
    }
    visible
}

fn turned_id() -> egui::Id {
    egui::Id::new("pc-quarter-turned-modes")
}

/// Tell the work area which monitors the desktop shows turned a quarter (90° or 270°): the pixel
/// size of each one's current mode, as the panel itself reports it (unturned). The desktop app
/// asks the Wayland compositor at startup; nobody else needs to call this.
pub fn set_quarter_turned_modes(ctx: &egui::Context, modes: Vec<[u32; 2]>) {
    ctx.data_mut(|d| d.insert_temp(turned_id(), std::sync::Arc::new(modes)));
}

/// The monitor's size the way the desktop shows it. winit 0.30 reports a Wayland output by its
/// mode, ignoring the output's transform: a portrait panel mounted sideways (1600 × 2560 px shown
/// as 2560 × 1600, common in handhelds) arrives as 800 × 1280 pt at scale 2 although the desktop
/// is 1280 × 800 pt. `reported` is that size in points, `pixels_per_point` what it was divided by,
/// `turned_modes` the modes from [`set_quarter_turned_modes`]: a monitor whose size in pixels is
/// one of them gets its width and height swapped. (Two monitors with the same mode, only one of
/// them turned, can't be told apart: both are swapped.)
pub fn oriented_monitor(reported: Vec2, pixels_per_point: f32, turned_modes: &[[u32; 2]]) -> Vec2 {
    let px = reported * pixels_per_point;
    let turned = turned_modes.iter().any(|&[w, h]| w != h && (px.x - w as f32).abs() <= 1.0 && (px.y - h as f32).abs() <= 1.0);
    if turned { vec2(reported.y, reported.x) } else { reported }
}

/// [`oriented_monitor`] with this context's scale and the modes it was told about.
fn oriented(ctx: &egui::Context, monitor: Vec2) -> Vec2 {
    match ctx.data(|d| d.get_temp::<std::sync::Arc<Vec<[u32; 2]>>>(turned_id())) {
        Some(turned) => oriented_monitor(monitor, ctx.pixels_per_point(), &turned),
        None => monitor,
    }
}

/// [`visible_part`] for this frame's viewport.
pub fn visible_rect(ctx: &egui::Context) -> Rect {
    let content = ctx.content_rect();
    let (monitor, inner, state) = ctx.input(|i| {
        let v = i.viewport();
        (v.monitor_size, v.inner_rect, WindowState::of(v))
    });
    visible_part(content, monitor.map(|m| oriented(ctx, m)), inner, state)
}

/// Does a window of `outer` size fit on a `monitor` (less the taskbar)?
pub fn fits(outer: Vec2, monitor: Vec2) -> bool {
    outer.x <= monitor.x + 0.5 && outer.y <= monitor.y - TASKBAR + 0.5
}

fn fitted_id() -> egui::Id {
    egui::Id::new("pc-window-fitted")
}

/// Once, as soon as the monitor is known: maximize a window that is bigger than its monitor
/// (it ran under the taskbar and off-screen). Returns true when it asked to.
pub fn fit_window(ctx: &egui::Context) -> bool {
    if ctx.data(|d| d.get_temp::<bool>(fitted_id())).is_some() {
        return false;
    }
    let content = ctx.content_rect();
    let (monitor, outer, maximized, fullscreen) = ctx.input(|i| {
        let v = i.viewport();
        (v.monitor_size, v.outer_rect, v.maximized, v.fullscreen)
    });
    // On Wayland the window's position, and so its outer rect, is never known (#3737): its content
    // size is the least the window takes up.
    let outer = outer.unwrap_or(content);
    let Some(m) = monitor else { return false };
    // A panel shown turned a quarter is reported unturned on Wayland: 800 × 900 "fits" 800 × 1280.
    let m = oriented(ctx, m);
    // macOS keeps new windows inside the screen's visible frame itself.
    if cfg!(target_os = "macos") {
        return false;
    }
    ctx.data_mut(|d| d.insert_temp(fitted_id(), true));
    if maximized == Some(true) || fullscreen == Some(true) || !m.is_finite() || !outer.is_finite() || fits(outer.size(), m) {
        return false;
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
    true
}

/// Room kept free for a second window's title bar. A [`egui::ViewportBuilder`] sizes the window's
/// content and the OS puts the title bar on top of that, which egui can't measure before the window
/// exists (the Windows title bar is 31 pt, the macOS one 28 pt, the Linux desktops' near that).
pub const TITLE_BAR: f32 = 32.0;
/// The least room [`fitted_window`] gives a window, however small the monitor claims to be.
pub const MIN_FITTED: Vec2 = vec2(320.0, 240.0);

/// The size and the minimum size for a second window that wants `wanted` and at least `min`, on a
/// `monitor` of that size: no wider than the monitor and no taller than what is left of it above
/// the [`TASKBAR`] and below the window's [`TITLE_BAR`]. A minimum the window can't have there is
/// lowered to its size, because a minimum larger than the screen forces the window off it. With no
/// monitor, or a nonsense one, both come back unchanged.
pub fn fitted_window(wanted: Vec2, min: Vec2, monitor: Option<Vec2>) -> (Vec2, Vec2) {
    let Some(m) = monitor else { return (wanted, min) };
    if !m.is_finite() || m.x < 1.0 || m.y < 1.0 {
        return (wanted, min);
    }
    let room = vec2(m.x, m.y - TASKBAR - TITLE_BAR).max(MIN_FITTED);
    let size = wanted.min(room);
    (size, min.min(size))
}

/// The size of the monitor the main window is on, the way the desktop shows it ([`oriented`]):
/// the one place [`fitted_builder`] gets it from.
fn monitor_of(ctx: &egui::Context) -> Option<Vec2> {
    let reported = ctx.input_for(egui::ViewportId::ROOT, |i| i.viewport().monitor_size);
    reported.map(|m| oriented(ctx, m))
}

/// `builder` with its size and minimum size fitted to the main window's monitor ([`fitted_window`]):
/// for the app's second windows, which nothing else keeps on the screen. A builder that asks for no
/// size is left alone. Not to be called inside a `ctx` closure (it reads the input).
pub fn fitted_builder(ctx: &egui::Context, mut builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
    let Some(wanted) = builder.inner_size else { return builder };
    let (size, min) = fitted_window(wanted, builder.min_inner_size.unwrap_or(Vec2::ZERO), monitor_of(ctx));
    builder.inner_size = Some(size);
    if builder.min_inner_size.is_some() {
        builder.min_inner_size = Some(min);
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;
    use WindowState::{FullScreen, Maximized, Normal};
    use egui::{Rect, pos2, vec2};

    const MONITOR: Vec2 = vec2(1366.0, 768.0);

    fn window(x: f32, y: f32, w: f32, h: f32) -> (Rect, Rect) {
        (Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h)), Rect::from_min_size(pos2(x, y), vec2(w, h)))
    }

    #[test]
    fn a_window_taller_than_its_display_is_cut_at_the_taskbar() {
        // 1440 × 900 at the desktop origin on 1366 × 768: right and bottom are off-screen.
        let (content, inner) = window(0.0, 0.0, 1440.0, 900.0);
        assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal), Rect::from_min_max(pos2(0.0, 0.0), pos2(1366.0, 768.0 - TASKBAR)));
        // Below a 31 pt title bar, a full-height window reaches under the taskbar.
        let (content, inner) = window(0.0, 31.0, 1366.0, 737.0);
        assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal).bottom(), 768.0 - TASKBAR - 31.0);
        // A small window clear of the taskbar is all visible.
        let (content, inner) = window(100.0, 80.0, 800.0, 500.0);
        assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal), content);
        // On a second monitor to the right, the same.
        let (content, inner) = window(1366.0 + 100.0, 0.0, 1440.0, 900.0);
        assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal).bottom(), 768.0 - TASKBAR);
    }

    #[test]
    fn maximized_unknown_or_nonsense_input_keeps_the_window() {
        let (content, inner) = window(0.0, 0.0, 1440.0, 900.0);
        assert_eq!(visible_part(content, Some(MONITOR), Some(inner), FullScreen), content, "full screen covers the taskbar on purpose");
        assert_eq!(visible_part(content, None, Some(inner), Normal), content);
        assert_eq!(visible_part(content, Some(vec2(f32::NAN, 768.0)), Some(inner), Normal), content);
        assert_eq!(visible_part(content, Some(vec2(0.0, 0.0)), Some(inner), Normal), content);
        assert_eq!(visible_part(content, Some(MONITOR), Some(Rect::from_min_size(pos2(f32::INFINITY, 0.0), vec2(10.0, 10.0))), Normal), content);
        // A guess that would leave almost nothing visible is not trusted.
        let (content, inner) = window(0.0, 600.0, 1000.0, 300.0);
        assert_eq!(visible_part(content, Some(MONITOR), Some(inner), Normal), content);
    }

    #[test]
    fn a_maximized_window_under_a_panel_is_still_cut_at_the_taskbar() {
        // #3737, X11 (Linux Mint): the window manager maximized the window to the whole 1920 × 1080
        // monitor without reserving the bottom panel's space, so the panel covers it.
        for scale in [1.0_f32, 1.5] {
            let monitor = vec2(1920.0, 1080.0) / scale;
            let content = Rect::from_min_size(pos2(0.0, 0.0), monitor);
            let bottom = monitor.y - TASKBAR;
            assert_eq!(visible_part(content, Some(monitor), Some(content), Maximized).bottom(), bottom, "at {scale}×");
            // A window manager that reserves the panel: the maximized window already ends above it.
            let fitted = Rect::from_min_size(pos2(0.0, 0.0), vec2(monitor.x, bottom));
            assert_eq!(visible_part(fitted, Some(monitor), Some(fitted), Maximized), fitted, "at {scale}×");
            // Full screen is meant to cover the panel.
            assert_eq!(visible_part(content, Some(monitor), Some(content), FullScreen), content, "at {scale}×");
        }
    }

    #[test]
    fn an_unknown_window_position_still_keeps_clear_of_the_taskbar() {
        // #3737, Wayland: only the monitor's size is known. A window taller than the monitor less
        // the taskbar can be seen at most down to there, wherever it is.
        let content = Rect::from_min_size(pos2(0.0, 0.0), vec2(1366.0, 740.0));
        assert_eq!(visible_part(content, Some(MONITOR), None, Normal), Rect::from_min_max(pos2(0.0, 0.0), pos2(1366.0, 768.0 - TASKBAR)));
        let big = Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 900.0));
        assert_eq!(visible_part(big, Some(MONITOR), None, Normal), Rect::from_min_max(pos2(0.0, 0.0), pos2(1366.0, 768.0 - TASKBAR)));
        // One that fits, or a maximized one, is all visible.
        let small = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 600.0));
        assert_eq!(visible_part(small, Some(MONITOR), None, Normal), small);
        assert_eq!(visible_part(content, Some(MONITOR), None, Maximized), content);
        assert_eq!(visible_part(content, Some(MONITOR), None, FullScreen), content);
    }

    fn frame(ctx: &egui::Context, monitor: Option<Vec2>, outer: Option<Rect>, maximized: Option<bool>) -> Vec<egui::ViewportCommand> {
        let mut input = egui::RawInput::default();
        if let Some(v) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
            v.monitor_size = monitor;
            v.outer_rect = outer;
            v.maximized = maximized;
        }
        let mut out = ctx.run_ui(input, |ui| {
            fit_window(ui.ctx());
        });
        out.textures_delta.clear();
        out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default()
    }

    #[test]
    fn a_window_too_big_for_its_display_is_maximized_once() {
        let big = Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1456.0, 939.0)));
        let ctx = egui::Context::default();
        // The monitor isn't known on the first frames: wait.
        assert!(!frame(&ctx, None, big, Some(false)).contains(&egui::ViewportCommand::Maximized(true)));
        let maximize = frame(&ctx, Some(MONITOR), big, Some(false)).contains(&egui::ViewportCommand::Maximized(true));
        assert_eq!(maximize, !cfg!(target_os = "macos"));
        // Once only: un-maximizing it later is the user's choice.
        assert!(!frame(&ctx, Some(MONITOR), big, Some(false)).contains(&egui::ViewportCommand::Maximized(true)));
        // A window that fits, or one already maximized, is left alone.
        let ctx = egui::Context::default();
        assert!(
            !frame(&ctx, Some(MONITOR), Some(Rect::from_min_size(pos2(40.0, 40.0), vec2(1200.0, 700.0))), Some(false))
                .contains(&egui::ViewportCommand::Maximized(true))
        );
        let ctx = egui::Context::default();
        assert!(!frame(&ctx, Some(MONITOR), big, Some(true)).contains(&egui::ViewportCommand::Maximized(true)));
        // Wayland: no outer rect, but a content area bigger than the monitor is maximized too (#3737).
        let ctx = egui::Context::default();
        let mut input = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 900.0))), ..Default::default() };
        if let Some(v) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
            v.monitor_size = Some(MONITOR);
            v.maximized = Some(false);
        }
        let mut out = ctx.run_ui(input, |ui| {
            fit_window(ui.ctx());
        });
        out.textures_delta.clear();
        let cmds = out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default();
        assert_eq!(cmds.contains(&egui::ViewportCommand::Maximized(true)), !cfg!(target_os = "macos"));
        assert!(fits(vec2(1366.0, 720.0), MONITOR) && !fits(vec2(1366.0, 740.0), MONITOR) && !fits(vec2(1440.0, 700.0), MONITOR));
    }

    /// What the Camera Raw window asks for: its size and its minimum size.
    const CAMERA_RAW: (Vec2, Vec2) = (vec2(1360.0, 900.0), vec2(900.0, 600.0));

    /// A fitted window is on the monitor, clear of the taskbar and its own title bar, and its
    /// minimum doesn't ask for more than it has.
    fn assert_on_screen(size: Vec2, min: Vec2, monitor: Vec2) {
        assert!(size.x <= monitor.x, "{size:?} is wider than {monitor:?}");
        assert!(size.y <= monitor.y - TASKBAR - TITLE_BAR, "{size:?} is too tall for {monitor:?}");
        assert!(min.x <= size.x && min.y <= size.y, "minimum {min:?} is larger than {size:?}");
    }

    #[test]
    fn a_second_window_larger_than_a_small_screen_is_fitted_to_it() {
        let (wanted, least) = CAMERA_RAW;
        // 1280 × 800: 1360 × 900 ran off the right edge and under the taskbar.
        let (size, min) = fitted_window(wanted, least, Some(vec2(1280.0, 800.0)));
        assert_on_screen(size, min, vec2(1280.0, 800.0));
        assert_eq!((size, min), (vec2(1280.0, 800.0 - TASKBAR - TITLE_BAR), least), "its minimum still fits");
        // 1366 × 768: wide enough, not tall enough.
        let (size, min) = fitted_window(wanted, least, Some(MONITOR));
        assert_on_screen(size, min, MONITOR);
        assert_eq!((size, min), (vec2(1360.0, 768.0 - TASKBAR - TITLE_BAR), least));
        // 1280 × 800 at a 125 % UI scale is 1024 × 640 points: the minimum has to give way too.
        let (size, min) = fitted_window(wanted, least, Some(vec2(1024.0, 640.0)));
        assert_on_screen(size, min, vec2(1024.0, 640.0));
        assert_eq!((size, min), (vec2(1024.0, 560.0), vec2(900.0, 560.0)));
        // A portrait screen: the width gives way, the height fits.
        let (size, min) = fitted_window(wanted, least, Some(vec2(800.0, 1280.0)));
        assert_on_screen(size, min, vec2(800.0, 1280.0));
        assert_eq!((size, min), (vec2(800.0, 900.0), vec2(800.0, 600.0)));
        // A window that fits is not made larger, and neither is its minimum.
        assert_eq!(fitted_window(vec2(800.0, 600.0), Vec2::ZERO, Some(MONITOR)), (vec2(800.0, 600.0), Vec2::ZERO));
    }

    #[test]
    fn a_large_unknown_or_nonsense_monitor_leaves_a_second_window_alone() {
        let (wanted, least) = CAMERA_RAW;
        assert_eq!(fitted_window(wanted, least, Some(vec2(2560.0, 1440.0))), CAMERA_RAW);
        assert_eq!(fitted_window(wanted, least, None), CAMERA_RAW);
        assert_eq!(fitted_window(wanted, least, Some(vec2(f32::NAN, 800.0))), CAMERA_RAW);
        assert_eq!(fitted_window(wanted, least, Some(vec2(1280.0, f32::NAN))), CAMERA_RAW);
        assert_eq!(fitted_window(wanted, least, Some(vec2(f32::INFINITY, 800.0))), CAMERA_RAW);
        assert_eq!(fitted_window(wanted, least, Some(vec2(0.0, 0.0))), CAMERA_RAW);
        assert_eq!(fitted_window(wanted, least, Some(vec2(1280.0, 0.0))), CAMERA_RAW);
        assert_eq!(fitted_window(wanted, least, Some(vec2(-1280.0, -800.0))), CAMERA_RAW);
    }

    #[test]
    fn a_tiny_monitor_does_not_fit_a_second_window_below_the_floor() {
        let (wanted, least) = CAMERA_RAW;
        // Shorter than the taskbar and the title bar together: the room would be negative.
        assert_eq!(fitted_window(wanted, least, Some(vec2(200.0, 60.0))), (MIN_FITTED, MIN_FITTED));
        assert_eq!(fitted_window(wanted, least, Some(vec2(1.0, 1.0))), (MIN_FITTED, MIN_FITTED));
        // Only the height is short.
        assert_eq!(fitted_window(wanted, least, Some(vec2(1280.0, 100.0))), (vec2(1280.0, MIN_FITTED.y), vec2(900.0, MIN_FITTED.y)));
        // The floor is room to fit into, not a size to grow to.
        assert_eq!(fitted_window(vec2(200.0, 100.0), vec2(100.0, 50.0), Some(vec2(200.0, 60.0))), (vec2(200.0, 100.0), vec2(100.0, 50.0)));
    }

    /// One frame of the main window on `monitor`, in which a second window's `builder` is fitted.
    fn fitted(ctx: &egui::Context, monitor: Option<Vec2>, builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
        let mut input = egui::RawInput::default();
        if let Some(v) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
            v.monitor_size = monitor;
        }
        let mut result = builder.clone();
        let mut out = ctx.run_ui(input, |ui| {
            result = fitted_builder(ui.ctx(), builder.clone());
        });
        out.textures_delta.clear();
        result
    }

    #[test]
    fn a_second_windows_builder_is_fitted_to_the_main_windows_monitor() {
        let (wanted, least) = CAMERA_RAW;
        let camera_raw = egui::ViewportBuilder::default().with_title("Camera Raw").with_inner_size(wanted).with_min_inner_size(least);
        let ctx = egui::Context::default();
        // Until the monitor is known the builder is as it was written.
        assert_eq!(fitted(&ctx, None, camera_raw.clone()), camera_raw);
        let b = fitted(&ctx, Some(vec2(1280.0, 800.0)), camera_raw.clone());
        assert_eq!((b.inner_size, b.min_inner_size), (Some(vec2(1280.0, 800.0 - TASKBAR - TITLE_BAR)), Some(least)));
        assert_eq!(b.title.as_deref(), Some("Camera Raw"), "the rest of the builder is kept");
        let b = fitted(&ctx, Some(vec2(1024.0, 640.0)), camera_raw.clone());
        assert_eq!((b.inner_size, b.min_inner_size), (Some(vec2(1024.0, 560.0)), Some(vec2(900.0, 560.0))));
        assert_eq!(fitted(&ctx, Some(vec2(2560.0, 1440.0)), camera_raw.clone()), camera_raw);
        // A builder with a size and no minimum (a document's extra window) gets no minimum.
        let document = egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]);
        let b = fitted(&ctx, Some(vec2(1024.0, 640.0)), document.clone());
        assert_eq!((b.inner_size, b.min_inner_size), (Some(vec2(800.0, 560.0)), None));
        assert_eq!(fitted(&ctx, Some(MONITOR), document.clone()), document);
        // A builder that asks for no size is left alone, its minimum included.
        let unsized_window = egui::ViewportBuilder::default().with_min_inner_size(least);
        assert_eq!(fitted(&ctx, Some(vec2(1024.0, 640.0)), unsized_window.clone()), unsized_window);
        // It is the main window's monitor that counts, also when asked from outside a frame.
        let b = fitted_builder(&ctx, camera_raw);
        assert_eq!(b.inner_size, Some(vec2(1024.0, 560.0)));
    }

    /// A handheld's portrait panel mounted sideways: 1600 × 2560 px, shown as 2560 × 1600 at
    /// scale 2 (a 1280 × 800 pt desktop). winit reports the unturned 800 × 1280 pt.
    const TURNED: [[u32; 2]; 1] = [[1600, 2560]];

    #[test]
    fn a_quarter_turned_panel_is_measured_the_way_it_is_shown() {
        assert_eq!(oriented_monitor(vec2(800.0, 1280.0), 2.0, &TURNED), vec2(1280.0, 800.0));
        // The same panel at another scale or UI zoom is still the same pixels.
        assert_eq!(oriented_monitor(vec2(640.0, 1024.0), 2.5, &TURNED), vec2(1024.0, 640.0));
        // A monitor that isn't turned, another monitor beside the turned one, a square one and no
        // list at all keep their size.
        assert_eq!(oriented_monitor(vec2(800.0, 1280.0), 2.0, &[]), vec2(800.0, 1280.0));
        assert_eq!(oriented_monitor(MONITOR, 1.0, &TURNED), MONITOR);
        assert_eq!(oriented_monitor(vec2(1280.0, 800.0), 2.0, &TURNED), vec2(1280.0, 800.0));
        assert_eq!(oriented_monitor(vec2(500.0, 500.0), 2.0, &[[1000, 1000]]), vec2(500.0, 500.0));
        // The window eframe opens there (1440 × 900 clamped to the unturned 800 pt width) "fits"
        // the reported monitor but not the real one.
        assert!(fits(vec2(800.0, 900.0), vec2(800.0, 1280.0)));
        assert!(!fits(vec2(800.0, 900.0), oriented_monitor(vec2(800.0, 1280.0), 2.0, &TURNED)));
    }

    #[test]
    fn a_window_too_tall_for_a_quarter_turned_panel_is_maximized() {
        let run = |turned: bool| {
            let ctx = egui::Context::default();
            if turned {
                set_quarter_turned_modes(&ctx, TURNED.to_vec());
            }
            let mut input = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 900.0))), ..Default::default() };
            if let Some(v) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
                v.native_pixels_per_point = Some(2.0);
                v.monitor_size = Some(vec2(800.0, 1280.0));
                v.maximized = Some(false);
            }
            let mut out = ctx.run_ui(input, |ui| {
                fit_window(ui.ctx());
            });
            out.textures_delta.clear();
            let cmds = out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default();
            cmds.contains(&egui::ViewportCommand::Maximized(true))
        };
        // Told the panel is turned, the 800 × 900 pt window doesn't fit the 1280 × 800 pt desktop.
        assert_eq!(run(true), !cfg!(target_os = "macos"));
        // Not told (X11, or an upright panel of that size), it fits and is left alone.
        assert!(!run(false));
    }

    #[test]
    fn menus_on_a_quarter_turned_panel_stop_at_its_real_bottom_edge() {
        let ctx = egui::Context::default();
        set_quarter_turned_modes(&ctx, TURNED.to_vec());
        let mut input = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 900.0))), ..Default::default() };
        if let Some(v) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
            v.native_pixels_per_point = Some(2.0);
            v.monitor_size = Some(vec2(800.0, 1280.0));
            v.maximized = Some(false);
        }
        let mut visible = Rect::NOTHING;
        let mut out = ctx.run_ui(input, |ui| {
            visible = visible_rect(ui.ctx());
        });
        out.textures_delta.clear();
        // Wayland gives no window position: at best the content starts at the monitor's top.
        assert_eq!(visible, Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 800.0 - TASKBAR)));
    }

    /// Camera Raw on that panel: fitted to the desktop as it is shown (1280 × 800 pt), not to the
    /// unturned 800 × 1280 pt the window library reports, which leaves it 900 pt tall there.
    #[test]
    fn a_second_window_on_a_quarter_turned_panel_is_fitted_to_the_desktop_as_shown() {
        let (wanted, least) = CAMERA_RAW;
        let run = |turned: bool| {
            let ctx = egui::Context::default();
            if turned {
                set_quarter_turned_modes(&ctx, TURNED.to_vec());
            }
            let camera_raw = egui::ViewportBuilder::default().with_inner_size(wanted).with_min_inner_size(least);
            let mut input = egui::RawInput::default();
            if let Some(v) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
                v.native_pixels_per_point = Some(2.0);
                v.monitor_size = Some(vec2(800.0, 1280.0));
            }
            let mut result = camera_raw.clone();
            let mut out = ctx.run_ui(input, |ui| {
                result = fitted_builder(ui.ctx(), camera_raw.clone());
            });
            out.textures_delta.clear();
            (result.inner_size, result.min_inner_size)
        };
        assert_eq!(run(true), (Some(vec2(1280.0, 800.0 - TASKBAR - TITLE_BAR)), Some(least)));
        assert_eq!(run(false), (Some(vec2(800.0, 900.0)), Some(vec2(800.0, 600.0))));
    }
}
