//! Self-logging must never change what the program does (ours, never offered upstream): the
//! window app, started from an empty settings folder and asked to quit, ends, and its log ends
//! with the line a run in order leaves.
//!
//! The first build that had the window-size check kept a strong handle on the window inside a
//! hook that lives as long as the egui context: after `app.quit` the window stayed on the screen
//! and the process never ended. This test fails on that build.
//!
//! It opens a real window, so it is ignored by default: run it on a machine with a screen,
//! `cargo test -p photocraft --test selflog_quit -- --ignored --nocapture`.
//! `PHOTOCRAFT_TEST_BIN=<program>` runs it against another build of the window app.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long the app may take to end once it has answered `app.quit`.
const QUIT_WITHIN: Duration = Duration::from_secs(15);

fn wait_for<T>(what: &str, within: Duration, mut ready: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(value) = ready() {
            return value;
        }
        assert!(start.elapsed() < within, "{what}: not within {within:?}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn last_lines(log: &Path, n: usize) -> String {
    let text = std::fs::read_to_string(log).unwrap_or_else(|e| format!("(no log at {}: {e})", log.display()));
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

#[test]
#[ignore = "opens a window: run with --ignored on a machine with a screen"]
fn a_plain_start_asked_to_quit_ends_in_order() {
    let dir: PathBuf = std::env::temp_dir().join(format!("photocraft-selflog-quit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (settings, token_file) = (dir.join("settings"), dir.join("control.token"));
    let log = settings.join("logs").join("photocraft.log");
    // A port nobody holds: ask the system for one and give it back.
    let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let program = std::env::var_os("PHOTOCRAFT_TEST_BIN").map_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_photocraft")), PathBuf::from);
    let mut app = Command::new(&program)
        .args(["--control", &port.to_string(), "--control-token-file"])
        .arg(&token_file)
        .env("PHOTOCRAFT_CONFIG_DIR", &settings)
        .env_remove("PHOTOCRAFT_CONTROL_PORT")
        .env_remove("PHOTOCRAFT_CONTROL_TOKEN")
        .env_remove("PHOTOCRAFT_CONTROL_TOKEN_FILE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Everything from here to the end must leave no window behind, also when it fails.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let token = wait_for("the control token file", Duration::from_secs(30), || std::fs::read_to_string(&token_file).ok().filter(|t| t.trim().len() == 64));
        let stream = wait_for("the control port", Duration::from_secs(30), || TcpStream::connect(("127.0.0.1", port)).ok());
        stream.set_read_timeout(Some(Duration::from_secs(70))).unwrap();
        let mut replies = BufReader::new(stream.try_clone().unwrap());
        let mut ask = |request: String| {
            let mut stream = &stream;
            writeln!(stream, "{request}").unwrap();
            let mut reply = String::new();
            replies.read_line(&mut reply).unwrap();
            assert!(reply.contains("\"ok\":true") || reply.contains("\"ok\": true"), "{request} was answered {reply}");
        };
        ask(format!("{{\"id\":\"auth\",\"method\":\"auth\",\"params\":{{\"token\":\"{}\"}}}}", token.trim()));
        // One request that the window answers from a frame, so the app is up and drawing.
        ask("{\"id\":1,\"method\":\"ui.inspect\",\"params\":{}}".to_owned());
        ask("{\"id\":2,\"method\":\"app.quit\",\"params\":{}}".to_owned());
    }));
    let asked = Instant::now();
    let ended = loop {
        match app.try_wait().unwrap() {
            Some(status) => break Some(status),
            None if asked.elapsed() >= QUIT_WITHIN => break None,
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };
    if ended.is_none() {
        let _ = app.kill();
        let _ = app.wait();
    }
    let tail = last_lines(&log, 12);
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
    let Some(status) = ended else {
        panic!("{} had not ended {QUIT_WITHIN:?} after app.quit (it was killed). The end of its log:\n{tail}", program.display());
    };
    assert!(status.success(), "ended with {status}. The end of its log:\n{tail}");
    assert!(tail.lines().last().is_some_and(|line| line.ends_with("run ended: in order")), "the log does not end with the closing line:\n{tail}");
}
