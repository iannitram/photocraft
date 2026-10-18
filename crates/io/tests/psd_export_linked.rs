//! PSD export embeds a linked smart object's file; a link that names something other than a
//! regular file (a crafted document's `/dev/zero`, a directory) must fail cleanly, not read
//! forever.
mod common;

use common::*;
use photocraft_color::{ColorMode, SampleType};
use photocraft_doc::*;
use photocraft_io::*;

fn linked_doc(path: &str) -> Document {
    let mut d = gen_doc(ColorMode::Rgb, SampleType::U8, Features::PIXELS);
    let cache = pattern(d.pixel_format(), photocraft_geom::Rect::new(1, 1, 6, 4), 7, true);
    d.layers.push(Layer::new(
        "Linked",
        LayerContent::Smart(SmartObject {
            contents_id: SmartContentsId::fresh(),
            source: SmartSource::Linked { path: path.to_string() },
            transform: photocraft_geom::Affine::IDENTITY,
            smart_filters: vec![],
            cache: Some(cache),
            psd_raw: None,
            filters_enabled: true,
            filter_mask: None,
            warp: None,
            stack_mode: None,
            perspective: None,
        }),
    ));
    d
}

/// The export either fails or warns, and says why; it never embeds what isn't a file. Two
/// guards can say it: `doc_path::resolve` (upstream's, via `craft_docpath`) refuses a device
/// path by name before anything opens it, and `read_linked` (ours) refuses anything left that is
/// not a regular file.
fn refuses(path: &str) {
    let said_why = |m: &str| m.contains("not a regular file") || m.contains("names a device, not a file");
    match export(&linked_doc(path), "a.psd", &ExportOptions::default()) {
        Err(e) => assert!(said_why(&e.to_string()), "{e}"),
        Ok(r) => {
            assert!(r.warnings.iter().any(|w| said_why(w)), "{:?}", r.warnings);
            assert!(!r.warnings.iter().any(|w| w.contains("was embedded")), "{:?}", r.warnings);
        }
    }
}

#[test]
fn a_directory_link_is_not_embedded() {
    refuses(&std::env::temp_dir().display().to_string());
}

#[cfg(unix)]
#[test]
fn a_device_link_is_not_read_forever() {
    refuses("/dev/zero");
}

#[test]
fn a_regular_linked_file_is_still_embedded() {
    let path = std::env::temp_dir().join(format!("photocraft-linked-{}.bin", std::process::id()));
    std::fs::write(&path, [1u8, 2, 3, 4]).unwrap();
    let r = export(&linked_doc(&path.display().to_string()), "a.psd", &ExportOptions::default());
    let _ = std::fs::remove_file(&path);
    let r = r.unwrap();
    assert!(r.warnings.iter().any(|w| w.contains("was embedded")), "{:?}", r.warnings);
}

/// Windows reports a named pipe as a regular file, and reading one blocks until its other end
/// writes: every way of naming one is refused before anything opens it. (These pipes don't
/// exist, so a path that slips through fails this test with "can't be read" instead of hanging.)
#[cfg(windows)]
#[test]
fn a_pipe_link_is_refused_by_name() {
    let pipe = format!("photocraft-no-such-pipe-{}", std::process::id());
    for path in [
        format!(r"\\.\pipe\{pipe}"),
        format!("//./pipe/{pipe}"),
        format!(r"\\?\pipe\{pipe}"),
        format!(r"\\?\GLOBALROOT\Device\NamedPipe\{pipe}"),
        format!(r"\\localhost\pipe\{pipe}"),
        format!(r"\\?\UNC\localhost\PIPE\{pipe}"),
    ] {
        refuses(&path);
    }
}

/// The verbatim form of a drive path (`\\?\C:\…`, what `canonicalize` returns) is an ordinary
/// file and is still embedded.
#[cfg(windows)]
#[test]
fn a_verbatim_drive_path_is_still_embedded() {
    let path = std::env::temp_dir().join(format!("photocraft-linked-verbatim-{}.bin", std::process::id()));
    std::fs::write(&path, [1u8, 2, 3, 4]).unwrap();
    let verbatim = std::fs::canonicalize(&path).unwrap();
    assert!(verbatim.display().to_string().starts_with(r"\\?\"), "{}", verbatim.display());
    let r = export(&linked_doc(&verbatim.display().to_string()), "a.psd", &ExportOptions::default());
    let _ = std::fs::remove_file(&path);
    let r = r.unwrap();
    assert!(r.warnings.iter().any(|w| w.contains("was embedded")), "{:?}", r.warnings);
}
