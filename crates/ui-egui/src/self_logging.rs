//! Ours (self-logging, never offered upstream; see `crates/craft-log`): what the window crate
//! adds to the app's own log and to Help › System Info, so that a session (or a person) does not
//! have to gather by hand what the app already knows.
//!
//! - [`APP`]: PhotoCraft as craft-log names it (the programs install their loggers with it).
//! - [`system_info_lines`]: the fields System Info lacked (the build's commit, the operating
//!   system, kernel, session, processor, memory, the screens and the window as the app found
//!   them, where the log is).
//! - [`decision_once`]: one `decision:` line at warn for a quiet second-best path, once per case.
//! - [`canvas_over_budget`]: the GPU-to-CPU fallback that used to leave no line at all.

use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};

use craft_log::report::{Report, mask_home};

/// PhotoCraft, as craft-log names it. The filter is upstream's default
/// (`apps/photocraft/src/logging.rs`): warnings from everything, info from our crates.
pub const APP: craft_log::App =
    craft_log::App { name: "photocraft", version: env!("CARGO_PKG_VERSION"), ours: "photocraft*", default_filter: "warn,photocraft*=info" };

static WINDOW_FACTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static DECIDED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// The app tells what it found when its window opened (the session the window is on, each
/// screen, the window's size asked for and got): the same lines it logged.
pub fn add_window_fact(line: impl Into<String>) {
    WINDOW_FACTS.lock().unwrap_or_else(PoisonError::into_inner).push(line.into());
}

/// The lines [`add_window_fact`] was given, in order.
pub fn window_facts() -> Vec<String> {
    WINDOW_FACTS.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// The diagnostic fields System Info shows after its own (English; they are for a report).
pub fn system_info_lines() -> Vec<String> {
    let report = Report::new(&APP);
    let mut lines = vec![format!("Build: {}", APP.build_line())];
    for (section, key, label) in [
        ("system", "operating system", "Operating system"),
        ("system", "kernel", "Kernel"),
        ("system", "session", "Desktop session"),
        ("processor", "model", "Processor"),
        ("processor", "threads", "Threads"),
        ("memory", "total", "Memory"),
        ("memory", "available", "Memory available"),
    ] {
        if let Some(value) = report.get(section, key) {
            lines.push(format!("{label}: {value}"));
        }
    }
    lines.extend(window_facts());
    lines.push(match craft_log::log_path() {
        Some(path) => format!("Log file: {}", mask_home(&path.display().to_string())),
        None => "Log file: none (standard error only)".to_owned(),
    });
    lines
}

/// Log `message()` at warn, once for each `case`: a quiet second-best path says so the first
/// time it is taken and stays silent after (it may be taken every frame).
pub fn decision_once(case: &str, message: impl FnOnce() -> String) {
    let mut decided = DECIDED.lock().unwrap_or_else(PoisonError::into_inner);
    if decided.get_or_insert_with(HashSet::new).insert(case.to_owned()) {
        // Built before the macro: `log::warn!` does not evaluate its arguments when warn is off.
        let text = message();
        log::warn!("{text}");
    }
}

/// A canvas whose display textures do not fit the GPU texture budget is drawn by the reduced CPU
/// canvas. Until now that left no line in the log.
pub fn canvas_over_budget(width: u32, height: u32, bytes: u64, budget: u64) {
    decision_once(&format!("gpu.canvas over texture budget {width}x{height} {bytes}"), || {
        format!(
            "decision: gpu.canvas: a {width} x {height} canvas needs {} MiB of display textures, over the GPU texture budget of {} MiB (a quarter of memory); drawing it on the CPU canvas",
            bytes >> 20,
            budget >> 20
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_info_gains_the_build_the_system_and_the_log() {
        add_window_fact("Window: on native Wayland");
        let lines = system_info_lines();
        assert!(lines.first().is_some_and(|l| l.starts_with("Build: photocraft ") && l.contains("commit ")), "{lines:?}");
        for label in ["Operating system: ", "Kernel: ", "Processor: ", "Memory: ", "Log file: "] {
            assert!(lines.iter().any(|l| l.starts_with(label)), "no {label} line in {lines:?}");
        }
        assert!(lines.iter().any(|l| l == "Window: on native Wayland"));
        // Nothing here names the user's home folder.
        if let Some(home) = std::env::var_os("HOME").and_then(|h| h.into_string().ok()).filter(|h| h.len() > 1) {
            assert!(lines.iter().filter(|l| l.starts_with("Log file: ")).all(|l| !l.contains(&home)), "{lines:?}");
        }
    }

    #[test]
    fn a_decision_is_said_once_for_each_case() {
        let said = std::cell::Cell::new(0);
        let say = || {
            said.set(said.get() + 1);
            "decision: test: once".to_owned()
        };
        decision_once("self-logging test case A", say);
        decision_once("self-logging test case A", say);
        decision_once("self-logging test case B", say);
        assert_eq!(said.get(), 2);
    }
}
