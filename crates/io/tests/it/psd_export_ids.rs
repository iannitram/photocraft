//! Numbering on PSD export: layer ids (`lyid`) and type layers' `Txt2` slots (`TextIndex`).

use photocraft_doc::text::{CharStyle, Kerning, TextRun};
use photocraft_doc::{Document, Layer, LayerContent, TextLayer};

fn doc() -> Document {
    Document::new("ids", photocraft_geom::Size::new(64, 32), photocraft_color::ColorMode::Rgb, photocraft_color::SampleType::U8)
}

fn text(d: &Document, s: &str, kerning: Kerning) -> TextLayer {
    let base = CharStyle { font_family: "Inter".into(), size_pt: 24.0, kerning, ..Default::default() };
    let mut t = TextLayer { text: s.into(), runs: vec![TextRun { len: s.len(), style: base }], ..Default::default() };
    t.sync_summary();
    photocraft_text::TextEngine::new().render_layer(&mut t, d.resolution_dpi, d.pixel_format());
    t
}

/// A type layer made in PhotoCraft must not take the `Txt2` slot of an imported one, even when it
/// comes first in the layer walk: the imported layer keeps its `TextIndex` (and with it its
/// Photoshop-written `Txt2` object), the new one gets the next free number.
#[test]
fn new_type_layer_does_not_take_an_imported_layers_text_index() {
    let mut d = doc();
    let fresh = text(&d, "NEW", Kerning::Metrics);
    let mut imported = text(&d, "AV", Kerning::Optical);
    let raw = photocraft_text::psd::build_tysh(&imported, d.resolution_dpi, None);
    imported.psd_raw = Some(std::sync::Arc::new(photocraft_text::psd::set_text_index(&raw, 0).unwrap()));
    // The new layer comes first in walk order.
    d.layers.push(Layer::new("new", LayerContent::Text(fresh)));
    d.layers.push(Layer::new("imported", LayerContent::Text(imported)));

    let bytes = photocraft_io::export(&d, "ids.psd", &Default::default()).unwrap().bytes;
    let back = photocraft_io::import("ids.psd", &bytes).unwrap().document;
    let index = |name: &str| {
        let l = back.layers.iter().find(|l| l.name == name).unwrap();
        let LayerContent::Text(t) = &l.content else { panic!("{name} is not text") };
        photocraft_text::psd::text_index(t.psd_raw.as_deref().unwrap())
    };
    assert_eq!(index("imported"), Some(0), "the imported layer lost its Txt2 slot");
    assert_eq!(index("new"), Some(1));
    // And the imported layer still finds its own object: Optical survives.
    let l = back.layers.iter().find(|l| l.name == "imported").unwrap();
    let LayerContent::Text(t) = &l.content else { panic!() };
    assert!(t.char_runs().iter().all(|r| r.style.kerning == Kerning::Optical), "imported layer read another layer's Txt2 object");
}

/// A kept layer id of `u32::MAX` (from a file or a .pcraft `psd_id`) must not overflow when group
/// dividers get their ids: every `lyid` stays unique and non-zero.
#[test]
fn group_divider_ids_after_a_max_layer_id_stay_unique() {
    let mut d = doc();
    let mut top = Layer::new("max", LayerContent::Raster(photocraft_raster::Surface::new(d.pixel_format())));
    top.psd_id = Some(u32::MAX);
    let mut inner = Layer::new("one", LayerContent::Raster(photocraft_raster::Surface::new(d.pixel_format())));
    inner.psd_id = Some(1);
    d.layers.push(Layer::group("g1", vec![inner]));
    d.layers.push(Layer::group("g2", vec![]));
    d.layers.push(top);

    let bytes = photocraft_io::export(&d, "ids.psd", &Default::default()).unwrap().bytes;
    let file = photocraft_psd::PsdFile::from_bytes(&bytes).unwrap();
    let ids: Vec<u32> = file.layers().iter().filter_map(|r| r.layer_id()).collect();
    assert_eq!(ids.len(), file.layers().len(), "every record carries an lyid");
    assert!(!ids.contains(&0), "{ids:?}");
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "duplicate lyid: {ids:?}");
    assert!(ids.contains(&u32::MAX) && ids.contains(&1), "kept ids must survive: {ids:?}");
}
