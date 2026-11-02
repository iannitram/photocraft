//! `photocraft-cli --version` names the commit a local build came from (#48). A build nobody
//! stamped must not print upstream's "(dev build)".

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const FLAGS: [&str; 2] = ["--version", "-V"];

/// A settings folder path that does not exist yet.
fn settings(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pc-cli-version-{}-{name}", std::process::id()))
}

fn run(settings: &Path, flag: &str) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_photocraft-cli"));
    cmd.arg(flag).env("PHOTOCRAFT_CONFIG_DIR", settings).stdin(Stdio::null());
    for name in ["CRAFT_LOG", "CRAFT_LOG_JSON", "CRAFT_LOG_STDERR", "RUST_LOG"] {
        cmd.env_remove(name);
    }
    cmd.output().unwrap()
}

fn stamped() -> bool {
    photocraft_engine::build_info::short_sha().is_some()
}

#[test]
fn the_version_flags_print_the_fork_text_and_exit_0() {
    for flag in FLAGS {
        let o = run(&settings("text"), flag);
        assert_eq!(o.status.code(), Some(0), "{flag}");
        assert_eq!(String::from_utf8_lossy(&o.stdout), format!("photocraft-cli {}\n", photocraft_cli::selflog::version_text()), "{flag}");
        assert!(o.stderr.is_empty(), "{flag}: {}", String::from_utf8_lossy(&o.stderr));
    }
}

#[test]
fn a_build_nobody_stamped_names_its_commit() {
    if stamped() {
        return;
    }
    for flag in FLAGS {
        let out = String::from_utf8_lossy(&run(&settings("commit"), flag).stdout).into_owned();
        assert!(out.starts_with(&format!("photocraft-cli {} (commit ", photocraft_engine::build_info::VERSION)), "{flag}: {out}");
        assert!(!out.contains("(dev build)"), "{flag}: {out}");
    }
}

#[test]
fn a_stamped_build_prints_upstreams_long_version() {
    if !stamped() {
        return;
    }
    let out = String::from_utf8_lossy(&run(&settings("stamped"), "--version").stdout).into_owned();
    assert_eq!(out, format!("photocraft-cli {}\n", photocraft_engine::build_info::long_version()));
}

#[test]
fn the_version_creates_no_settings_folder_and_no_log() {
    for flag in FLAGS {
        let dir = settings("quiet");
        assert_eq!(run(&dir, flag).status.code(), Some(0), "{flag}");
        assert!(!dir.exists(), "{flag} created the settings folder");
    }
}
