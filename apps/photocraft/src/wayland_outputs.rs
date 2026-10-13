//! Which Wayland outputs the compositor shows turned a quarter (Linux).
//!
//! winit 0.30 reports a Wayland monitor's size from its current mode and ignores the output's
//! transform (`MonitorHandle::size`, `platform_impl/linux/wayland/output.rs`). A portrait panel
//! mounted sideways, as most handhelds have (Legion Go: 1600 × 2560 px shown as 2560 × 1600),
//! therefore arrives as 800 × 1280 pt at scale 2 although the desktop is 1280 × 800 pt. eframe
//! clamps the first window to that (1440 × 900 became 800 × 900), and `work_area::fit_window`
//! found that it "fits": the window opened with its bottom 150 pt below the screen and under the
//! panel, and was never maximized.
//!
//! winit doesn't expose the transform, so we ask the compositor ourselves, once, over a second
//! short-lived connection, before the window opens: `wl_output.geometry` carries the transform and
//! `wl_output.mode` the size winit will report. [`quarter_turned_modes`] is what
//! `photocraft_ui_egui::work_area::set_quarter_turned_modes` takes.
//!
//! Only the events are interpreted here; the decision is pure and tested on every platform. A
//! compositor that can't be reached (X11, no `WAYLAND_DISPLAY`) yields nothing and the app starts
//! as before. Outputs that are turned or plugged in later aren't tracked.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

/// One output: the pixel size of its current mode as the panel reports it (unturned), and
/// whether the compositor shows it turned by 90° or 270°.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    pub mode: Option<[u32; 2]>,
    pub quarter_turned: bool,
}

/// The modes of the outputs shown turned a quarter (no duplicates). A square mode looks the same
/// either way and is left out.
pub fn quarter_turned_modes(outputs: &[Output]) -> Vec<[u32; 2]> {
    let mut modes: Vec<[u32; 2]> = Vec::new();
    for output in outputs {
        if let (true, Some([w, h])) = (output.quarter_turned, output.mode)
            && w != h
            && !modes.contains(&[w, h])
        {
            modes.push([w, h]);
        }
    }
    modes
}

/// A mode's size from `wl_output.mode` (signed on the wire): `None` unless both are positive.
pub fn mode_size(width: i32, height: i32) -> Option<[u32; 2]> {
    let (w, h) = (u32::try_from(width).ok()?, u32::try_from(height).ok()?);
    (w > 0 && h > 0).then_some([w, h])
}

#[cfg(target_os = "linux")]
mod query {
    use super::{Output, mode_size};
    use wayland_client::protocol::{wl_output, wl_registry};
    use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

    #[derive(Default)]
    struct State {
        outputs: Vec<Output>,
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, (): &(), _: &Connection, qh: &QueueHandle<Self>) {
            if let wl_registry::Event::Global { name, interface, version } = event
                && interface == "wl_output"
            {
                let index = state.outputs.len();
                state.outputs.push(Output::default());
                // `geometry` and `mode` are in every version; 2 is as far as this needs to go.
                let _output: wl_output::WlOutput = registry.bind(name, version.min(2), qh, index);
            }
        }
    }

    impl Dispatch<wl_output::WlOutput, usize> for State {
        fn event(state: &mut Self, _: &wl_output::WlOutput, event: wl_output::Event, index: &usize, _: &Connection, _: &QueueHandle<Self>) {
            let Some(output) = state.outputs.get_mut(*index) else { return };
            match event {
                wl_output::Event::Geometry { transform, .. } => {
                    use wl_output::Transform as T;
                    output.quarter_turned = matches!(transform, WEnum::Value(T::_90 | T::_270 | T::Flipped90 | T::Flipped270));
                }
                wl_output::Event::Mode { flags: WEnum::Value(flags), width, height, .. } if flags.contains(wl_output::Mode::Current) => {
                    output.mode = mode_size(width, height);
                }
                _ => {}
            }
        }
    }

    /// Every output the compositor advertises, or `None` when there is no Wayland compositor to
    /// ask (or it hung up). Two round trips: the first delivers the globals (and sends our
    /// binds), the second each bound output's geometry and mode.
    pub fn outputs() -> Option<Vec<Output>> {
        let conn = Connection::connect_to_env().ok()?;
        let mut queue = conn.new_event_queue::<State>();
        let qh = queue.handle();
        let _registry = conn.display().get_registry(&qh, ());
        let mut state = State::default();
        queue.roundtrip(&mut state).ok()?;
        queue.roundtrip(&mut state).ok()?;
        Some(state.outputs)
    }
}

/// Whether the compositor may be asked over a second connection of our own; `var` reads an
/// environment variable. Only on an ordinary Wayland desktop, where `WAYLAND_DISPLAY` names the
/// compositor's socket and anyone may connect to it again. Not when `WAYLAND_SOCKET` is set: that
/// is one connection, already open and handed to this process, which `connect_to_env` would take
/// and the window library then could not have. And not under gamescope (Steam's Game Mode, known
/// by `XDG_CURRENT_DESKTOP` or `GAMESCOPE_WAYLAND_DISPLAY`), which shows the one app full screen
/// and is not a desktop to fit a window to.
pub fn may_ask_compositor(var: impl Fn(&str) -> Option<String>) -> bool {
    let set = |name: &str| var(name).is_some_and(|v| !v.is_empty());
    // Not `SteamGamepadUI`: Steam also sets it for what it starts from Big Picture on an ordinary
    // desktop, where the compositor is the desktop's own and is to be asked.
    let gamescope =
        set("GAMESCOPE_WAYLAND_DISPLAY") || var("XDG_CURRENT_DESKTOP").is_some_and(|d| d.split(':').any(|part| part.trim().eq_ignore_ascii_case("gamescope")));
    set("WAYLAND_DISPLAY") && !set("WAYLAND_SOCKET") && !gamescope
}

/// [`quarter_turned_modes`] of the running Wayland compositor's outputs; empty when there is
/// none to ask, or it is not to be asked ([`may_ask_compositor`]).
#[cfg(target_os = "linux")]
pub fn quarter_turned_modes_now() -> Vec<[u32; 2]> {
    if !may_ask_compositor(|name| std::env::var(name).ok()) {
        return Vec::new();
    }
    let outputs = query::outputs().unwrap_or_default();
    let modes = quarter_turned_modes(&outputs);
    if !modes.is_empty() {
        log::info!("Wayland outputs shown turned a quarter (mode sizes in px): {modes:?}");
    }
    modes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_quarter_turned_outputs_with_a_known_mode_are_listed() {
        let handheld = Output { mode: Some([1600, 2560]), quarter_turned: true };
        let upright = Output { mode: Some([2560, 1440]), quarter_turned: false };
        let no_mode = Output { mode: None, quarter_turned: true };
        let square = Output { mode: Some([1000, 1000]), quarter_turned: true };
        assert_eq!(quarter_turned_modes(&[handheld]), vec![[1600, 2560]]);
        assert_eq!(quarter_turned_modes(&[upright, handheld, no_mode, square, handheld]), vec![[1600, 2560]]);
        assert!(quarter_turned_modes(&[upright, no_mode, square]).is_empty());
        assert!(quarter_turned_modes(&[]).is_empty());
    }

    #[test]
    fn the_compositor_is_asked_only_on_an_ordinary_wayland_desktop() {
        let ask = |vars: &[(&str, &str)]| {
            let vars: Vec<(String, String)> = vars.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
            may_ask_compositor(|name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()))
        };
        // A Plasma or GNOME Wayland session.
        assert!(ask(&[("WAYLAND_DISPLAY", "wayland-0"), ("XDG_CURRENT_DESKTOP", "KDE")]));
        assert!(ask(&[("WAYLAND_DISPLAY", "wayland-1"), ("XDG_CURRENT_DESKTOP", "ubuntu:GNOME"), ("WAYLAND_SOCKET", "")]));
        // X11, or nothing at all: there is no compositor to ask.
        assert!(!ask(&[("DISPLAY", ":0"), ("XDG_CURRENT_DESKTOP", "KDE")]));
        assert!(!ask(&[("WAYLAND_DISPLAY", "")]));
        assert!(!ask(&[]));
        // A connection handed over by the parent is the window library's, never ours.
        assert!(!ask(&[("WAYLAND_DISPLAY", "wayland-0"), ("WAYLAND_SOCKET", "7")]));
        assert!(!ask(&[("WAYLAND_SOCKET", "7")]));
        // gamescope, by either of its signs.
        assert!(!ask(&[("WAYLAND_DISPLAY", "gamescope-0"), ("XDG_CURRENT_DESKTOP", "gamescope")]));
        assert!(!ask(&[("WAYLAND_DISPLAY", "wayland-0"), ("XDG_CURRENT_DESKTOP", "KDE:Gamescope")]));
        assert!(!ask(&[("WAYLAND_DISPLAY", "wayland-0"), ("GAMESCOPE_WAYLAND_DISPLAY", "gamescope-0")]));
        // Started by Steam from Big Picture on an ordinary desktop: still a desktop to ask.
        assert!(ask(&[("WAYLAND_DISPLAY", "wayland-0"), ("XDG_CURRENT_DESKTOP", "KDE"), ("SteamGamepadUI", "1")]));
    }

    #[test]
    fn a_mode_needs_two_positive_sizes() {
        assert_eq!(mode_size(1600, 2560), Some([1600, 2560]));
        assert_eq!(mode_size(0, 2560), None);
        assert_eq!(mode_size(1600, -1), None);
    }
}
