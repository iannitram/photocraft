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
/// Arguments whose value is a secret: the value is never logged, as `--flag value` or as
/// `--flag=value`. (`--control-token-file` names a file and is not one.)
pub const SECRET_ARGUMENTS: &[&str] = &["--control-token"];
/// Environment switches whose value is a secret: the report names them and never shows them.
pub const SECRET_SWITCHES: &[&str] = &["PHOTOCRAFT_CONTROL_TOKEN"];

/// The settings folder, resolved the way the window app resolves it: `PHOTOCRAFT_CONFIG_DIR`,
/// else `<exe dir>/PhotoCraftData` when a portable marker sits beside the program, else the
/// platform's folder. `None` when no user folder is known.
pub fn settings_dir(env: impl Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>) -> Option<PathBuf> {
    let set = |key: &str| env(key).filter(|v| !v.is_empty()).map(PathBuf::from);
    set("PHOTOCRAFT_CONFIG_DIR")
        .or_else(|| exe_dir.filter(|d| PORTABLE_MARKERS.iter().any(|m| d.join(m).is_file())).map(|d| d.join(PORTABLE_DATA_DIR)))
        .or_else(|| {
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

/// Whether the run only prints the usage or the version (`--help`, `-h`, `help`, `--version`,
/// `version`, `<subcommand> --help`): such a run installs no logger, so it creates no settings
/// folder and no log, as in the window app. Decided the way [`crate::run`] decides it; a usage
/// error is a failure and is logged.
pub fn leaves_no_trace(args: &[OsString]) -> bool {
    let Some(args) = args.iter().map(|a| a.to_str().map(str::to_owned)).collect::<Option<Vec<String>>>() else { return false };
    let Some((first, rest)) = args.split_first() else { return false };
    if matches!(first.as_str(), "-h" | "--help" | "help" | "--version" | "version") {
        return true;
    }
    crate::SUBCOMMANDS.iter().find(|s| s.name == first).is_some_and(|sub| matches!(crate::parse(sub, rest), Ok(None)))
}

/// Install the logger for this run. Call in `main` once [`leaves_no_trace`] has said no. The
/// crate masks the value of every [`SECRET_ARGUMENTS`] in the command line it logs and in every
/// later line, and every argument that [`looks_like_token`] wherever it stands (a stray argument,
/// `--other=<token>`). The crate also hides `PHOTOCRAFT_CONTROL_TOKEN`'s value wherever it
/// appears (its name holds TOKEN).
pub fn install() {
    let dir = this_settings_dir().map(|d| d.join("logs"));
    craft_log::cli::install_checked(&APP, dir.as_deref(), SECRET_ARGUMENTS, &looks_like_token);
}

/// Whether `value` is a control token: exactly 64 hexadecimal digits, the whole of its form
/// (`photocraft_automation::security::validate_token`).
pub fn looks_like_token(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `text` with every control token hidden: the line `serve` prints with the token it made up
/// for the launch, and any run of exactly 64 hexadecimal digits.
pub fn hide_tokens(text: &str) -> String {
    const SAID: &str = "photocraft-cli control token:";
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| if line.trim_start().starts_with(SAID) { format!("{SAID} {}", craft_log::cli::HIDDEN) } else { hide_hex_runs(line) })
        .collect();
    lines.join("\n")
}

fn hide_hex_runs(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_hexdigit() {
            let run = bytes[i..].iter().take_while(|b| b.is_ascii_hexdigit()).count();
            if run == 64 {
                out.push_str(&line[start..i]);
                out.push_str(craft_log::cli::HIDDEN);
                start = i + run;
            }
            i += run;
        } else {
            i += 1;
        }
    }
    out.push_str(&line[start..]);
    out
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
/// `log: <path>` as its last line on standard error. A control token in that text is hidden
/// ([`hide_tokens`]): `serve` prints the one it makes up before it binds, and a failing bind
/// would otherwise copy it into the log.
pub fn finish(code: i32, said: &str) {
    if code != 0 {
        let said = hide_tokens(said);
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
    r.add_switches(SWITCH_PREFIXES, SECRET_SWITCHES);
    r.add_helpers();
    let file = |name: &str| logs.as_ref().map(|d| d.join(name)).filter(|p| p.is_file());
    let (window, previous, cli) = (file(WINDOW_LOG_FILE), file(PREVIOUS_WINDOW_LOG_FILE), file(&craft_log::cli::file_name(APP.name)));
    let crash = logs.as_ref().and_then(|d| newest_file(d, &|name| name.starts_with("crash")));
    r.add_logs(
        logs.as_deref(),
        &[("window log", window.as_deref()), ("previous window log", previous.as_deref()), ("command-line log", cli.as_deref())],
        crash.as_deref(),
    );
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
        let env = env_of(&[
            ("PHOTOCRAFT_CONFIG_DIR", "/tmp/pc-run-7"),
            ("HOME", "/home/u"),
            ("XDG_CONFIG_HOME", "/home/u/.cfg"),
            ("APPDATA", "C:/Users/u/AppData/Roaming"),
        ]);
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
        assert!(writeln!(err, "warning: a font is missing").is_ok());
        assert!(writeln!(err, "photocraft-cli: cannot open the file: it is empty").is_ok());
        assert_eq!(err.text(), "warning: a font is missing\nphotocraft-cli: cannot open the file: it is empty");
        // Only the end of a long output is kept; all of it still reaches standard error.
        let long = "x".repeat(REASON_BYTES * 2);
        assert!(err.write_all(long.as_bytes()).is_ok());
        assert_eq!(err.text().len(), REASON_BYTES);
        drop(err);
        assert_eq!(seen.len(), "warning: a font is missing\n".len() + "photocraft-cli: cannot open the file: it is empty\n".len() + REASON_BYTES * 2);
    }

    fn os(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn help_and_the_version_leave_no_trace_and_everything_else_is_logged() {
        for quiet in [&["--help"][..], &["-h"], &["help"], &["--version"], &["version"], &["--version", "extra"]] {
            assert!(leaves_no_trace(&os(quiet)), "{quiet:?}");
        }
        for sub in crate::SUBCOMMANDS {
            assert!(leaves_no_trace(&os(&[sub.name, "--help"])), "{} --help", sub.name);
            assert!(leaves_no_trace(&os(&[sub.name, "a.png", "-h"])), "{} a.png -h", sub.name);
        }
        // A flag's value is not a request for help, and help after a usage error is still the error.
        assert!(leaves_no_trace(&os(&["serve", "--control-token", "0123", "-h"])));
        assert!(!leaves_no_trace(&os(&["serve", "--control-token", "--help"])));
        assert!(!leaves_no_trace(&os(&["convert", "--fromat", "png", "--help"])));
        for logged in
            [&[][..], &["frobnicate"], &["frobnicate", "--help"], &["convert", "a.png", "b.png"], &["serve", "--port", "1"], &["diagnostics"], &["info"]]
        {
            assert!(!leaves_no_trace(&os(logged)), "{logged:?}");
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            // Not Unicode: a usage error whatever else is on the line (`run_os`), so it is logged.
            assert!(!leaves_no_trace(&[OsString::from("--help"), OsString::from_vec(vec![0x66, 0xff])]));
        }
    }

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn a_token_in_what_the_run_said_is_hidden() {
        let said = format!("photocraft-cli control token: {TOKEN}\nerror: I/O: bind 127.0.0.1:7878: Address already in use");
        assert_eq!(hide_tokens(&said), "photocraft-cli control token: ***\nerror: I/O: bind 127.0.0.1:7878: Address already in use");
        assert_eq!(hide_tokens(&format!("unknown argument `{TOKEN}`")), "unknown argument `***`");
        // Only a run of exactly 64 digits: a 63- or 65-digit run, a commit or a port stays.
        let near = format!("{} {}0 b3309e27 7878", &TOKEN[1..], TOKEN);
        assert_eq!(hide_tokens(&near), near);
        assert_eq!(hide_tokens("héllo wörld"), "héllo wörld");
    }

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    /// The command line as the crate logs it, with this call site's secrets.
    fn masked(args: &[&str]) -> (Vec<String>, Vec<String>) {
        craft_log::cli::mask_args_checked(&strings(args), SECRET_ARGUMENTS, &looks_like_token, &[])
    }

    #[test]
    fn a_token_is_hidden_wherever_it_stands_on_the_command_line() {
        const OTHER: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
        let (shown, secrets) = masked(&["photocraft-cli", "serve", TOKEN, "--frobnicate"]);
        assert_eq!(shown, ["photocraft-cli", "serve", "***", "--frobnicate"]);
        assert_eq!(secrets, [TOKEN]);
        let other = format!("--other={TOKEN}");
        assert_eq!(masked(&["photocraft-cli", "mcp", &other]).0, ["photocraft-cli", "mcp", "--other=***"]);
        assert_eq!(masked(&["photocraft-cli", "serve", "--control-token", TOKEN]).0, ["photocraft-cli", "serve", "--control-token", "***"]);
        // Two different tokens side by side: both hidden.
        assert_eq!(masked(&["photocraft-cli", "serve", TOKEN, OTHER]).0, ["photocraft-cli", "serve", "***", "***"]);
        // Only exactly 64 digits.
        assert!(!looks_like_token(&TOKEN[1..]) && !looks_like_token(&format!("{TOKEN}0")) && !looks_like_token("b3309e27"));
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
