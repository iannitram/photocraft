#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

fn main() {
    // `args_os`, not `args`: a non-Unicode argument (a Latin-1 file name on Linux) must be a
    // usage error, not a panic before anything is printed (issue #1108).
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    photocraft_cli::selflog::install();
    if let Some(json) = photocraft_cli::selflog::wants_diagnostics(&args) {
        std::process::exit(photocraft_cli::selflog::diagnostics(json, &mut std::io::stdout()));
    }
    let mut said = photocraft_cli::selflog::Capture::new(std::io::stderr());
    let code = photocraft_cli::run_os(&args, &mut std::io::stdout(), &mut said);
    photocraft_cli::selflog::finish(code, &said.text());
    std::process::exit(code);
}
