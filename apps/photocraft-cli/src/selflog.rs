//! Self-logging for the command-line program. **Ours, never offered upstream** (see
//! `crates/craft-log`; every commit title starts `self-logging:`).
//!
//! Before this the command-line program installed no logger at all: every `log::` line from a
//! run was dropped. Now every run appends to `photocraft-cli.log` in the same `logs` folder as
//! the window app's `photocraft.log`, with the build line first; a run that exits with an error
//! logs what it printed on standard error and ends with `log: <path>`; and
//! `photocraft-cli diagnostics [--json]` prints the diagnostic report. Standard output and the
//! rest of standard error are unchanged (`CRAFT_LOG_STDERR=1` copies log lines there).

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};

use craft_log::report::{Report, mask_home, newest_file};

/// What craft-log needs to know about PhotoCraft. The filter is the window app's own default
/// (`apps/photocraft/src/logging.rs`): warnings from everything, info from our crates.
pub const APP: craft_log::App =
    craft_log::App { name: "photocraft", version: env!("CARGO_PKG_VERSION"), ours: "photocraft*", default_filter: "warn,photocraft*=info" };

/// The window app's portable-mode markers and data folder (`apps/photocraft/src/app_dirs.rs`).
const PORTABLE_MARKERS: [&str; 2] = ["portable.txt", "PhotoCraft.portable"];
const PORTABLE_DATA_DIR: &str = "PhotoCraftData";
/// The window app's log and its predecessor (upstream's names), and ours for this program.
pub const WINDOW_LOG_FILE: &str = "photocraft.log";
pub const PREVIOUS_WINDOW_LOG_FILE: &str = "photocraft.1.log";
/// Environment switches PhotoCraft reads besides craft-log's own.
pub const SWITCH_PREFIXES: &[&str] = &["PHOTOCRAFT_", "WGPU_"];
/// The most of a failing run's standard error that is copied into the log.
const REASON_BYTES: usize = 4096;

/// The settings folder, resolved the way the window app resolves it: `PHOTOCRAFT_CONFIG_DIR`,
/// else `<exe dir>/PhotoCraftData` when a portable marker sits beside the program, else the
/// platform's folder. `None` when no user folder is known.
pub fn settings_dir(env: impl Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>) -> Option<PathBuf> {
    let set = |key: &str| env(key).filter(|v| !v.is_empty()).map(PathBuf::from);
    set("PHOTOCRAFT_CONFIG_DIR").or_else(|| exe_dir.filter(|d| PORTABLE_MARKERS.iter().any(|m| d.join(m).is_file())).map(|d| d.join(PORTABLE_DATA_DIR))).or_else(|| {
        if cfg!(target_os = "macos") {
            set("HOME").map(|h| h.join("Library/Application Support/Photocraft"))
        } else if cfg!(windows) {
            set("APPDATA").map(|a| a.join("Photocraft"))
        } else {
            set("XDG_CONFIG_HOME").or_else(|| set("HOME").map(|h| h.join(".config"))).map(|c| c.join("photocraft"))
        }
    })
}

/// The `logs` folder the window app writes `photocraft.log` into ([`settings_dir`]`/logs`).
/// `None` when no user folder is known; the run then has no log file.
pub fn log_dir(env: impl Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>) -> Option<PathBuf> {
    settings_dir(env, exe_dir).map(|d| d.join("logs"))
}

fn this_settings_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok();
    settings_dir(|key| std::env::var_os(key), exe.as_deref().and_then(Path::parent))
}

/// Install the logger for this run. Call first thing in `main`.
pub fn install() {
    let dir = this_settings_dir().map(|d| d.join("logs"));
    craft_log::cli::install(&APP, dir.as_deref());
}

/// Standard error, with the end of what was written kept for the log.
pub struct Capture<W: Write> {
    inner: W,
    kept: Vec<u8>,
}

impl<W: Write> Capture<W> {
    pub fn new(inner: W) -> Self {
        Capture { inner, kept: Vec::new() }
    }

    /// The last [`REASON_BYTES`] written, as text.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.kept).trim().to_owned()
    }
}

impl<W: Write> Write for Capture<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.kept.extend_from_slice(buf.get(..n).unwrap_or(buf));
        if self.kept.len() > REASON_BYTES {
            let cut = self.kept.len() - REASON_BYTES;
            self.kept.drain(..cut);
        }
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Call with the exit code and what the run printed on standard error, just before exiting: a
/// failing run logs that text whole (so the log holds why, not only the code) and prints
/// `log: <path>` as its last line on standard error.
pub fn finish(code: i32, said: &str) {
    if code != 0 {
        let reason = if said.is_empty() { format!("exit code {code}") } else { format!("exit code {code}: {said}") };
        craft_log::cli::failed(&APP, &reason);
    }
}

/// Whether the arguments ask for the diagnostic report (`diagnostics [--json]`), and as JSON.
pub fn wants_diagnostics(args: &[OsString]) -> Option<bool> {
    let mut it = args.iter().map(|a| a.as_os_str());
    if it.next() != Some(OsStr::new("diagnostics")) {
        return None;
    }
    match (it.next(), it.next()) {
        (None, _) => Some(false),
        (Some(flag), None) if flag == OsStr::new("--json") => Some(true),
        _ => None,
    }
}

/// The diagnostic report as this program can know it: what craft-log gathers (build, system,
/// processor, memory), the folders, the switches in force, and where the logs are. Graphics,
/// the screens and the window are known only to the window app, whose log starts with them.
pub fn report() -> Report {
    let settings = this_settings_dir();
    let logs = settings.as_ref().map(|d| d.join("logs"));
    let show = |p: Option<&Path>| p.map_or_else(|| craft_log::report::UNKNOWN.to_owned(), |p| mask_home(&p.display().to_string()));
    let mut r = Report::new(&APP);
    r.add("graphics", "adapter", "known to the window app only: its log's \"graphics:\" line");
    r.add("screens", "screens and window", "known to the window app only: its log's \"screen\" and \"window:\" lines");
    r.add("folders", "settings", show(settings.as_deref()));
    r.add("folders", "preferences", show(settings.as_ref().map(|d| d.join("preferences.json")).as_deref()));
    r.add_switches(SWITCH_PREFIXES);
    r.add_helpers();
    let file = |name: &str| logs.as_ref().map(|d| d.join(name)).filter(|p| p.is_file());
    let (window, previous, cli) = (file(WINDOW_LOG_FILE), file(PREVIOUS_WINDOW_LOG_FILE), file(&craft_log::cli::file_name(APP.name)));
    let crash = logs.as_ref().and_then(|d| newest_file(d, &|name| name.starts_with("crash")));
    r.add_logs(logs.as_deref(), &[("window log", window.as_deref()), ("previous window log", previous.as_deref()), ("command-line log", cli.as_deref())], crash.as_deref());
    r
}

/// Print the diagnostic report on `out` (text, or JSON). The exit code.
pub fn diagnostics(json: bool, out: &mut dyn Write) -> i32 {
    let r = report();
    let text = if json { r.json() } else { r.text() };
    match writeln!(out, "{}", text.trim_end()) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
        let map: HashMap<String, OsString> = pairs.iter().map(|(k, v)| ((*k).to_owned(), OsString::from(v))).collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn the_override_wins_and_the_logs_sit_in_its_logs_folder() {
        let env = env_of(&[("PHOTOCRAFT_CONFIG_DIR", "/tmp/pc-run-7"), ("HOME", "/home/u"), ("XDG_CONFIG_HOME", "/home/u/.cfg"), ("APPDATA", "C:/Users/u/AppData/Roaming")]);
        assert_eq!(log_dir(env, None), Some(PathBuf::from("/tmp/pc-run-7").join("logs")));
    }

    #[test]
    fn without_an_override_it_is_the_window_apps_folder() {
        let env = env_of(&[("PHOTOCRAFT_CONFIG_DIR", ""), ("HOME", "/home/u"), ("APPDATA", "C:/Users/u/AppData/Roaming")]);
        let expected = if cfg!(target_os = "macos") {
            PathBuf::from("/home/u").join("Library/Application Support/Photocraft")
        } else if cfg!(windows) {
            PathBuf::from("C:/Users/u/AppData/Roaming").join("Photocraft")
        } else {
            PathBuf::from("/home/u/.config/photocraft")
        };
        assert_eq!(log_dir(env, None), Some(expected.join("logs")));
        if cfg!(all(unix, not(target_os = "macos"))) {
            let env = env_of(&[("HOME", "/home/u"), ("XDG_CONFIG_HOME", "/home/u/.cfg")]);
            assert_eq!(log_dir(env, None), Some(PathBuf::from("/home/u/.cfg/photocraft/logs")));
        }
    }

    #[test]
    fn a_portable_marker_beside_the_program_moves_the_logs_with_the_settings() {
        let dir = std::env::temp_dir().join(format!("photocraft-cli-selflog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());
        let env = env_of(&[("HOME", "/home/u"), ("APPDATA", "C:/Users/u/AppData/Roaming")]);
        // No marker: the platform folder.
        assert_ne!(log_dir(&env, Some(&dir)), Some(dir.join(PORTABLE_DATA_DIR).join("logs")));
        assert!(std::fs::write(dir.join("portable.txt"), b"").is_ok());
        assert_eq!(log_dir(&env, Some(&dir)), Some(dir.join(PORTABLE_DATA_DIR).join("logs")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_user_folder_means_no_log_file() {
        assert_eq!(log_dir(env_of(&[]), None), None);
        assert_eq!(log_dir(env_of(&[("HOME", ""), ("APPDATA", ""), ("XDG_CONFIG_HOME", "")]), None), None);
    }

    #[test]
    fn standard_error_passes_through_and_its_end_is_kept_for_the_log() {
        let mut seen = Vec::new();
        let mut err = Capture::new(&mut seen);
        assert!(write!(err, "warning: a font is missing\n").is_ok());
        assert!(write!(err, "photocraft-cli: cannot open the file: it is empty\n").is_ok());
        assert_eq!(err.text(), "warning: a font is missing\nphotocraft-cli: cannot open the file: it is empty");
        // Only the end of a long output is kept; all of it still reaches standard error.
        let long = "x".repeat(REASON_BYTES * 2);
        assert!(err.write_all(long.as_bytes()).is_ok());
        assert_eq!(err.text().len(), REASON_BYTES);
        drop(err);
        assert_eq!(seen.len(), "warning: a font is missing\n".len() + "photocraft-cli: cannot open the file: it is empty\n".len() + REASON_BYTES * 2);
    }

    #[test]
    fn diagnostics_is_asked_for_by_name_only() {
        let args = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(wants_diagnostics(&args(&["diagnostics"])), Some(false));
        assert_eq!(wants_diagnostics(&args(&["diagnostics", "--json"])), Some(true));
        assert_eq!(wants_diagnostics(&args(&["diagnostics", "--yaml"])), None);
        assert_eq!(wants_diagnostics(&args(&["info", "diagnostics"])), None);
        assert_eq!(wants_diagnostics(&args(&[])), None);
    }

    #[test]
    fn the_report_ends_with_where_the_logs_are() {
        let r = report();
        let text = r.text();
        assert!(text.contains("photocraft"), "{text}");
        assert!(r.get("logs", "folder").is_some());
        assert!(r.get("folders", "settings").is_some());
        let mut out = Vec::new();
        assert_eq!(diagnostics(true, &mut out), 0);
        assert!(serde_json::from_slice::<serde_json::Value>(&out).is_ok(), "the --json form is JSON");
    }
}
