# Cloud sandbox facts

From the setup probe on 2026-10-12 (Default Cloud Environment). The probe stalled before timing the build, so the first nightly run should add the build and test timings here.

| | |
|---|---|
| Machine | Ubuntu 24.04, 4 cores (Xeon 2.8 GHz), 15 GB RAM, no swap, ~28 GB free disk. No GPU; Mesa software rasterizers only (`swrast`) |
| Rust | 1.97 preinstalled with rustup; the workspace needs >= 1.95 and pins no toolchain. `rustup target add wasm32-unknown-unknown` takes ~4 s |
| crates | `cargo fetch` works (~15 s for the whole workspace). Bare `curl` to crates.io gets 403, which doesn't matter |
| System packages | No missing-package error seen up to the probe's stall (the build was well into the egui/wgpu crates) |
| GitHub, our fork | `git push` works, including to `main` (fast-forward of 18 commits accepted). `gh auth status` says the token is invalid, but `gh api` calls work for the fork (admin rights) |
| GitHub, other repos | Repos not attached to the session: the API and `codeload.github.com` tarballs are refused ("GitHub access to this repository is not enabled for this session"); `git ls-remote` / `git clone` still work. Upstream is now attached to the nightly routine for this reason |
| Corpus | Hosts in `xtask/src/corpus_pins.rs`: github.com (git works, tarballs may not), www.schaik.com (200), raw.pixls.us (200). `cargo xtask corpus --all` not tried yet |
| Waiting | Never start a long build in the background and end the turn to wait: nothing wakes an unattended run. Run builds and tests in the foreground with a long timeout |
| Build timings | TBD (first nightly run) |
