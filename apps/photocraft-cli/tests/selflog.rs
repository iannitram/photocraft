//! Process-level tests of the command-line program's self-logging (ours, never offered upstream):
//! what a run leaves behind, and what it must never write down.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const OTHER_TOKEN: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("pc-cli-selflog-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The program with its settings folder at `settings` and no log switches from the caller.
fn run(settings: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_photocraft-cli"));
    cmd.args(args).env("PHOTOCRAFT_CONFIG_DIR", settings).stdin(Stdio::null());
    for name in ["CRAFT_LOG", "CRAFT_LOG_JSON", "CRAFT_LOG_STDERR", "RUST_LOG", "PHOTOCRAFT_CONTROL_TOKEN", "PHOTOCRAFT_CONTROL_TOKEN_FILE"] {
        cmd.env_remove(name);
    }
    cmd.envs(env.iter().copied()).output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn log_of(settings: &Path) -> PathBuf {
    settings.join("logs").join("photocraft-cli.log")
}

#[test]
fn help_and_the_version_create_no_folder_and_no_log() {
    let d = tmp("quiet");
    let settings = d.join("settings");
    let quiet: &[&[&str]] = &[&["--version"], &["version"], &["--help"], &["-h"], &["help"], &["convert", "--help"], &["serve", "-h"], &["mcp", "--control-token", TOKEN, "--help"]];
    for args in quiet {
        let o = run(&settings, args, &[]);
        assert_eq!(o.status.code(), Some(0), "{args:?}: {}", text(&o.stderr));
        assert!(!o.stdout.is_empty() && o.stderr.is_empty(), "{args:?}: {}", text(&o.stderr));
        assert!(!settings.exists(), "{args:?} created the settings folder");
    }
    assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0, "nothing was written beside it either");

    // The same folder is where a failing run does leave its log, so the checks above looked in
    // the right place.
    let o = run(&settings, &["frobnicate"], &[]);
    assert_eq!(o.status.code(), Some(2));
    let log = std::fs::read_to_string(log_of(&settings)).unwrap();
    assert!(log.contains("command line: ") && log.contains("unknown command `frobnicate`"), "{log}");
    assert!(text(&o.stderr).trim_end().ends_with(&format!("log: {}", log_of(&settings).display())), "{}", text(&o.stderr));
    let _ = std::fs::remove_dir_all(&d);
}

/// Logging must never change how a run exits: an argument that is not Unicode stays upstream's
/// usage error (their issue 1108), and a standard error that cannot be written to stays the
/// run's own exit code. Both were a panic (exit 101) through the shared crate's own install and
/// its `log:` line.
#[cfg(target_os = "linux")]
#[test]
fn the_logger_does_not_change_how_a_run_exits() {
    use std::os::unix::ffi::OsStringExt;
    let d = tmp("exits");
    let settings = d.join("settings");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_photocraft-cli"));
    cmd.arg("info").arg(std::ffi::OsString::from_vec(vec![b'f', 0xff, b'.', b'p', b's', b'd'])).env("PHOTOCRAFT_CONFIG_DIR", &settings).stdin(Stdio::null());
    let o = cmd.output().unwrap();
    assert_eq!(o.status.code(), Some(2), "{}", text(&o.stderr));
    assert!(text(&o.stderr).contains("is not valid Unicode"), "{}", text(&o.stderr));
    let log = std::fs::read_to_string(log_of(&settings)).unwrap();
    assert!(log.contains("command line: ") && log.contains("is not valid Unicode"), "{log}");

    let full = std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_photocraft-cli"));
    cmd.arg("frobnicate").env("PHOTOCRAFT_CONFIG_DIR", &settings).stdin(Stdio::null()).stdout(Stdio::null()).stderr(full);
    assert_eq!(cmd.status().unwrap().code(), Some(2));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_control_token_is_never_written_down() {
    let d = tmp("token");
    let settings = d.join("settings");
    // Usage errors (an unknown flag after the token), so nothing is served and the run is logged;
    // both spellings of the flag, plain and JSON lines, and the copy on standard error.
    let o = run(&settings, &["serve", "--control-token", TOKEN, "--frobnicate"], &[("CRAFT_LOG_STDERR", "1")]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o.stderr));
    assert!(text(&o.stderr).contains("serve --control-token *** --frobnicate"), "{}", text(&o.stderr));
    assert!(!text(&o.stderr).contains(TOKEN) && !text(&o.stdout).contains(TOKEN));
    let equals = format!("--control-token={TOKEN}");
    let o = run(&settings, &["mcp", &equals, "--frobnicate"], &[("CRAFT_LOG_JSON", "1"), ("PHOTOCRAFT_CONTROL_TOKEN", OTHER_TOKEN)]);
    assert_eq!(o.status.code(), Some(2), "{}", text(&o.stderr));
    assert!(!text(&o.stderr).contains(TOKEN) && !text(&o.stdout).contains(TOKEN));

    // The report names the token's environment switch and does not show it.
    for json in [&["diagnostics"][..], &["diagnostics", "--json"]] {
        let o = run(&settings, json, &[("PHOTOCRAFT_CONTROL_TOKEN", OTHER_TOKEN)]);
        assert_eq!(o.status.code(), Some(0), "{}", text(&o.stderr));
        let report = text(&o.stdout);
        assert!(report.contains("PHOTOCRAFT_CONTROL_TOKEN") && report.contains("set (***)"), "{report}");
        assert!(!report.contains(OTHER_TOKEN) && !text(&o.stderr).contains(OTHER_TOKEN), "{report}");
    }

    let log = std::fs::read_to_string(log_of(&settings)).unwrap();
    assert!(log.contains("serve --control-token *** --frobnicate"), "{log}");
    assert!(log.contains("mcp --control-token=*** --frobnicate"), "{log}");
    assert!(!log.contains(TOKEN) && !log.contains(OTHER_TOKEN), "a token is in the log:\n{log}");
    for entry in std::fs::read_dir(settings.join("logs")).unwrap() {
        let file = std::fs::read(entry.unwrap().path()).unwrap();
        assert!(!text(&file).contains(TOKEN) && !text(&file).contains(OTHER_TOKEN));
    }
    let _ = std::fs::remove_dir_all(&d);
}
