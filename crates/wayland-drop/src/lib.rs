//! Files dropped onto a winit window on Wayland.
//!
//! winit 0.30 implements no Wayland drag and drop: `WindowEvent::DroppedFile` never arrives there
//! (winit #4729 adds it to 0.30.x and is not merged yet). A drop is delivered to the window's own
//! `wl_surface`, so it can only be seen on winit's own connection: [`Listener::start`] adopts that
//! connection (the one `unsafe` call of this crate, in `linux.rs`) and listens on its own event
//! queue for drops of `text/uri-list` (what Dolphin, Nautilus and every other file manager offer).
//!
//! The URI-list parsing is pure and tested on every platform; on anything but Linux the listener
//! never starts.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;

/// What the listener reports.
#[derive(Debug, Clone, PartialEq)]
pub enum Drop {
    /// Files dropped at `at` (surface coordinates, logical points).
    Files { paths: Vec<PathBuf>, at: (f64, f64) },
    /// Something was dropped that is not a list of files (text, an image from a browser):
    /// nothing was opened, and the caller may say so.
    NotFiles,
}

/// A running listener. Dropping it (or [`Listener::stop`]) stops and joins its thread; it must
/// stop before winit's display goes away (see `linux.rs`).
pub struct Listener {
    // Held for its `Drop`, which stops and joins the thread.
    #[cfg(target_os = "linux")]
    _inner: linux::Inner,
}

impl Listener {
    /// Start listening on the Wayland display `display` (the `wl_display` pointer of winit's
    /// connection, from the window's raw display handle). `notify` runs on the listener thread
    /// after each [`Drop`] is queued (to wake the UI). `None` off Linux, or when the compositor
    /// offers no drag and drop.
    ///
    /// The caller guarantees what the safety argument in `linux.rs` needs: `display` is winit's
    /// live connection, and this listener is stopped (dropped) before that connection closes.
    pub fn start(display: std::ptr::NonNull<std::ffi::c_void>, notify: impl Fn() + Send + 'static) -> Option<(Self, std::sync::mpsc::Receiver<Drop>)> {
        #[cfg(target_os = "linux")]
        {
            linux::start(display, Box::new(notify)).map(|(inner, rx)| (Listener { _inner: inner }, rx))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (display, notify);
            None
        }
    }

    /// Stop the listener thread and wait for it (at most about one poll interval).
    pub fn stop(self) {}
}

/// The MIME type of a list of files.
pub const URI_LIST: &str = "text/uri-list";

/// The local paths in a `text/uri-list` (RFC 2483): one URI per line, `#` lines are comments, only
/// `file:` URIs on this machine (no host, or `localhost`) count, and `%XX` escapes are decoded to
/// the bytes of the path (so spaces and non-ASCII names come back as they are on disk).
pub fn paths_from_uri_list(list: &str) -> Vec<PathBuf> {
    list.lines().map(|l| l.trim_end_matches('\r').trim()).filter(|l| !l.is_empty() && !l.starts_with('#')).filter_map(file_uri_path).collect()
}

fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://").or_else(|| uri.strip_prefix("FILE://"))?;
    let path = if rest.starts_with('/') {
        rest
    } else {
        // file://host/path: only this machine.
        let (host, _) = rest.split_once('/')?;
        if !host.eq_ignore_ascii_case("localhost") {
            return None;
        }
        &rest[host.len()..]
    };
    let bytes = percent_decode(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(bytes).ok().map(PathBuf::from)
    }
}

/// `%XX` escapes decoded; `None` for a broken escape.
fn percent_decode(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = b.get(i + 1..i + 3)?;
            let v = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uri_list_gives_the_local_paths_as_on_disk() {
        let list = "# dropped from a file manager\r\nfile:///home/u/Pictures/Holiday%20photo.psd\r\nfile:///home/u/Bilder/K%C3%B6ln%20%E2%80%94%20Dom.jpg\r\nfile://localhost/tmp/a.png\r\n\r\nhttps://example.com/b.png\r\nsmb://nas/share/c.png\r\nfile://otherhost/tmp/d.png\r\n";
        assert_eq!(
            paths_from_uri_list(list),
            vec![PathBuf::from("/home/u/Pictures/Holiday photo.psd"), PathBuf::from("/home/u/Bilder/Köln — Dom.jpg"), PathBuf::from("/tmp/a.png")]
        );
    }

    #[test]
    fn broken_or_empty_lists_give_nothing() {
        assert!(paths_from_uri_list("").is_empty());
        assert!(paths_from_uri_list("# only a comment\n").is_empty());
        assert!(paths_from_uri_list("file:///tmp/bad%2").is_empty(), "a broken escape is skipped");
        assert!(paths_from_uri_list("file:///tmp/bad%zz.png").is_empty());
        assert_eq!(paths_from_uri_list("file:///a\nfile:///b"), vec![PathBuf::from("/a"), PathBuf::from("/b")], "no trailing newline");
    }

    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_utf8_keeps_its_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let p = paths_from_uri_list("file:///tmp/caf%E9.png");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].as_os_str().as_bytes(), b"/tmp/caf\xe9.png");
    }
}
