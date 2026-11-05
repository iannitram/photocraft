//! `photocraft --version` names the commit a local build came from (#48). A build nobody stamped
//! must not print upstream's "(dev build)". The real binary runs, and every case exits before the
//! window opens.

use std::process::{Command, Output, Stdio};

const FLAGS: [&[&str]; 3] = [&["--version"], &["-V"], &["--version", "--safegpu"]];

fn run(args: &[&str]) -> Output {
    let binary = std::env::var_os("NEXTEST_BIN_EXE_photocraft").unwrap_or_else(|| env!("CARGO_BIN_EXE_photocraft").into());
    let dir = std::env::temp_dir().join(format!("photocraft-version-{}", std::process::id()));
    let mut cmd = Command::new(binary);
    cmd.args(args).env("PHOTOCRAFT_CONFIG_DIR", dir).stdin(Stdio::null());
    for name in ["PHOTOCRAFT_CONTROL_PORT", "PHOTOCRAFT_CONTROL_TOKEN", "PHOTOCRAFT_CONTROL_TOKEN_FILE", "CRAFT_LOG", "RUST_LOG"] {
        cmd.env_remove(name);
    }
    cmd.output().unwrap()
}

fn stdout(args: &[&str]) -> String {
    let o = run(args);
    assert_eq!(o.status.code(), Some(0), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stamped() -> bool {
    photocraft_engine::build_info::short_sha().is_some()
}

#[test]
fn a_build_nobody_stamped_names_its_commit() {
    if stamped() {
        return;
    }
    for args in FLAGS {
        let out = stdout(args);
        assert!(out.starts_with(&format!("photocraft {} (commit ", photocraft_engine::build_info::VERSION)), "{args:?}: {out}");
        assert!(out.ends_with(")\n") && !out.contains("(dev build)"), "{args:?}: {out}");
    }
}

#[test]
fn a_stamped_build_prints_upstreams_long_version() {
    if !stamped() {
        return;
    }
    for args in FLAGS {
        assert_eq!(stdout(args), format!("photocraft {}\n", photocraft_engine::build_info::long_version()), "{args:?}");
    }
}

#[test]
fn the_version_is_one_line_and_names_the_program() {
    for args in FLAGS {
        let out = stdout(args);
        assert!(out.starts_with("photocraft ") && out.matches('\n').count() == 1, "{args:?}: {out}");
    }
}
