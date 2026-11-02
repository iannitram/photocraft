use super::filters::{smart, smart_mut};
use super::placing::{detach_psd, placement, set_placement};
use super::rendering::{refresh, refresh_or_fail, unapplied_filters};
use super::source::{authorized_source_bytes, embedded_source_bytes, source_bytes};
use super::{
    Arc, DocId, Document, EngineError, Homography, LayerContent, LayerId, Metadata, Rect, Result, Session, Size, SmartContentsId, SmartLink, SmartSource,
    Surface, Value, bad, base_name, decode_source, encode_source, json, layer_param, other,
};

pub(super) fn path_param<'a>(cmd: &str, p: &'a Value) -> Result<&'a str> {
    p.get("path").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or_else(|| bad(cmd, "pass `path`"))
}

/// Instances sharing contents in this document, including those inside groups.
fn instances(doc: &Document, contents_id: SmartContentsId) -> Vec<LayerId> {
    doc.walk().into_iter().filter_map(|(_, _, l)| matches!(&l.content, LayerContent::Smart(sm) if sm.contents_id == contents_id).then_some(l.id)).collect()
}

/// Swaps shared contents, keeping each instance's transform and filters, and re-renders.
pub(super) fn set_source(
    s: &mut Session,
    p: &Value,
    label: &str,
    keep_psd: bool,
    make: impl FnOnce(&Metadata, &SmartSource) -> Result<SmartSource>,
) -> Result<Value> {
    let id = layer_param(s, p)?;
    let doc = &s.active().ok_or(EngineError::NoDocument)?.doc;
    let new = make(&doc.metadata, &smart(doc, id)?.source)?;
    set_shared_source(s, id, label, keep_psd, new, None)
}

// Keep the shared-instance transaction out of the per-command source-builder instantiations.
// `refit` maps the new source's pixels onto the old source's frame (Replace Contents with `fit`):
// each instance's placement is composed with it, so the new contents land where the old ones were.
fn set_shared_source(s: &mut Session, id: LayerId, label: &str, keep_psd: bool, new: SmartSource, refit: Option<Homography>) -> Result<Value> {
    let (parent, contents_id, changed, warnings) = s.edit(label, |doc, _| {
        let contents_id = smart(doc, id)?.contents_id;
        let changed = instances(doc, contents_id);
        let mut warnings = std::collections::BTreeSet::new();
        for &instance in &changed {
            let sm = smart_mut(doc, instance)?;
            sm.source = new.clone();
            if let Some(map) = &refit {
                // A warp is a grid over the old source's pixels: it has no meaning on another size.
                if sm.warp.is_some() {
                    return Err(other("a warped smart object can't refit new contents: replace them without `fit`, or remove the warp first"));
                }
                let placed = placement(sm).mul(map);
                set_placement(sm, placed);
            }
            if !keep_psd {
                detach_psd(doc.layer_mut(instance).ok_or(EngineError::NoLayer(instance))?);
            }
            warnings
                .extend(unapplied_filters(smart(doc, instance)?).into_iter().map(|n| format!("the {n} smart filter is not supported, so it was not applied")));
            refresh_or_fail(doc, instance)?;
        }
        Ok((doc.id, contents_id, changed, warnings))
    })?;
    // An old editor must not overwrite newly replaced/relinked contents on close. Keep its
    // document open, but the next Edit Contents opens the replacement source.
    s.smart_links.retain(|l| l.parent != parent || l.contents_id != contents_id);
    // Every instance changed, not only `layer`: list them, so a caller doesn't repeat the
    // command once per copy (#4075).
    Ok(json!({"layer": id.0, "instances": changed.iter().map(|l| l.0).collect::<Vec<_>>(), "warnings": warnings}))
}

pub(super) fn replace_contents(s: &mut Session, p: &Value) -> Result<Value> {
    let path = path_param("layer.smartObjects.replaceContents", p)?.to_string();
    let bytes = photocraft_format::read_file(std::path::Path::new(&path)).map_err(|e| other(format!("can't read {path}: {e}")))?;
    let id = layer_param(s, p)?;
    let fit = match p.get("fit") {
        None | Some(Value::Null) => ReplaceFit::Keep,
        Some(v) => match v.as_str() {
            Some("keep") => ReplaceFit::Keep,
            Some("cover") => ReplaceFit::Cover,
            Some("contain") => ReplaceFit::Contain,
            _ => return Err(bad("layer.smartObjects.replaceContents", format!("`fit` is \"keep\", \"cover\" or \"contain\", not {v}"))),
        },
    };
    replace_contents_fit(s, id, &base_name(&path), bytes, fit)
}

/// What Replace Contents does with contents whose pixel size differs from the old ones'.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplaceFit {
    /// Photoshop's behaviour: every instance keeps its scale, so its size on the canvas changes.
    Keep,
    /// Scale the new contents, centred, until they cover the frame the old ones filled. What
    /// overflows on one axis is left to the layer's mask or the canvas edge.
    Cover,
    /// Scale the new contents, centred, until they fit inside that frame.
    Contain,
}

/// The map from the new contents' pixels onto the old contents' frame (`old` and `new`: pixel
/// sizes), or `None` when there is nothing to change.
fn refit_map(old: Size, new: Size, fit: ReplaceFit) -> Option<Homography> {
    if fit == ReplaceFit::Keep || old == new || new.width == 0 || new.height == 0 {
        return None;
    }
    let (ow, oh, nw, nh) = (f64::from(old.width), f64::from(old.height), f64::from(new.width), f64::from(new.height));
    let k = if fit == ReplaceFit::Cover { (ow / nw).max(oh / nh) } else { (ow / nw).min(oh / nh) };
    Some(Homography([k, 0.0, (ow - k * nw) / 2.0, 0.0, k, (oh - k * nh) / 2.0, 0.0, 0.0, 1.0]))
}

/// Replace Contents with a file the caller already read, for the smart object `p` names (`layer`,
/// else the active layer): the body of the command minus its ambient path read (#8061).
pub fn replace_contents_from_bytes(s: &mut Session, p: &Value, file_name: &str, bytes: Vec<u8>) -> Result<Value> {
    let id = layer_param(s, p)?;
    replace_contents_bytes(s, id, file_name, bytes)
}

/// Replace Contents from a file's name and bytes: what the desktop and web file pickers hand over.
pub fn replace_contents_bytes(s: &mut Session, id: LayerId, file_name: &str, bytes: Vec<u8>) -> Result<Value> {
    replace_contents_fit(s, id, file_name, bytes, ReplaceFit::Keep)
}

/// [`replace_contents_bytes`] with a choice of what happens when the new contents have another
/// pixel size. The result names both sizes and the layer's bounds before and after, and warns
/// when the layer changed size on the canvas (the step where a template goes wrong quietly).
pub fn replace_contents_fit(s: &mut Session, id: LayerId, file_name: &str, bytes: Vec<u8>, fit: ReplaceFit) -> Result<Value> {
    let new_size = decode_source(file_name, &bytes)?.size; // fail before touching the document
    let doc = &s.active().ok_or(EngineError::NoDocument)?.doc;
    let old = smart(doc, id)?;
    // Only contents held in memory are measured: a linked file is not read from disk for this.
    let old_size = embedded_source_bytes(&doc.metadata, &old.source).and_then(|(name, bytes)| decode_source(&name, &bytes).ok()).map(|d| d.size);
    let old_bounds = old.cache.as_ref().map(Surface::content_bounds);
    let refit = match (fit, old_size) {
        (ReplaceFit::Keep, _) => None,
        (_, Some(old_size)) => refit_map(old_size, new_size, fit),
        (_, None) => return Err(other("the old contents can't be read, so there is no frame to fit the new ones to: replace them without `fit`")),
    };
    let refitted = refit.is_some();
    let source = SmartSource::Embedded { file_name: file_name.to_string(), bytes: Arc::new(bytes) };
    let mut out = set_shared_source(s, id, "Replace Contents", false, source, refit)?;
    let new_bounds = s.active().and_then(|st| smart(&st.doc, id).ok()).and_then(|sm| sm.cache.as_ref().map(Surface::content_bounds));
    let rect = |r: Rect| json!([r.x0, r.y0, r.x1, r.y1]);
    out["contentsSize"] = json!([new_size.width, new_size.height]);
    if let Some(o) = old_size {
        out["previousContentsSize"] = json!([o.width, o.height]);
    }
    if let Some(b) = new_bounds {
        out["bounds"] = rect(b);
    }
    if let Some(b) = old_bounds {
        out["previousBounds"] = rect(b);
    }
    if let (Some(o), false) = (old_size.filter(|o| *o != new_size), refitted) {
        let covers = match (old_bounds, new_bounds) {
            (Some(was), Some(now)) => format!(" and now covers {} x {} px on the canvas (was {} x {})", now.width(), now.height(), was.width(), was.height()),
            _ => String::new(),
        };
        // After upstream's unapplied-filter warnings from `set_shared_source`, not instead of them.
        let note = format!(
            "the new contents are {} x {} px, the old were {} x {} px: the layer keeps its scale{covers}",
            new_size.width, new_size.height, o.width, o.height
        );
        match out["warnings"].as_array_mut() {
            Some(list) => list.push(json!(note)),
            None => out["warnings"] = json!([note]),
        }
    }
    Ok(out)
}

/// The file name and bytes of smart object `id`'s contents.
pub fn contents_of(doc: &Document, id: LayerId) -> Result<(String, Arc<Vec<u8>>)> {
    source_bytes(&doc.metadata, &smart(doc, id)?.source).ok_or_else(|| other("the smart object's contents are unavailable"))
}

/// Writes the smart object's file as it is. To a path with another extension (a `.png` for
/// `.psb` or `.pcraft` contents) the contents are opened and exported in that format instead.
pub(super) fn export_contents(s: &mut Session, p: &Value) -> Result<Value> {
    let path = path_param("layer.smartObjects.exportContents", p)?;
    let id = layer_param(s, p)?;
    let (name, bytes) = contents_of(&s.active().ok_or(EngineError::NoDocument)?.doc, id)?;
    let ext = |f: &str| {
        let e = std::path::Path::new(f).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
        e.map(|e| match e.as_str() {
            "jpeg" => "jpg".to_string(),
            "tiff" => "tif".to_string(),
            _ => e,
        })
    };
    if ext(path).is_none() || ext(path) == ext(&name) {
        crate::file_cmds::write_file(path, &bytes)?;
        return Ok(json!({"path": path, "fileName": name, "bytes": bytes.len(), "converted": false}));
    }
    let contents = decode_source(&name, &bytes)?;
    let r = photocraft_io::export(&contents, path, &Default::default()).map_err(|e| other(format!("can't export the contents as {}: {e}", base_name(path))))?;
    crate::file_cmds::write_file(path, &r.bytes)?;
    Ok(json!({"path": path, "fileName": name, "bytes": r.bytes.len(), "converted": true, "warnings": r.warnings}))
}

pub(super) fn edit_contents(s: &mut Session, p: &Value) -> Result<Value> {
    let id = layer_param(s, p)?;
    let st = s.active().ok_or(EngineError::NoDocument)?;
    let parent = st.doc.id;
    let contents_id = smart(&st.doc, id)?.contents_id;
    // Already open for editing: switch to that document rather than opening another copy.
    if let Some(index) =
        s.smart_links.iter().find(|l| l.parent == parent && l.contents_id == contents_id).and_then(|l| s.documents().iter().position(|d| d.doc.id == l.child))
    {
        s.set_active(index);
        return Ok(json!({"document": index, "parentLayer": id.0}));
    }
    let source = &smart(&st.doc, id)?.source;
    let (name, bytes) = authorized_source_bytes(s, "layer.smartObjects.editContents", &st.doc.metadata, source)?
        .ok_or_else(|| other("the smart object's contents are unavailable"))?;
    let mut child = decode_source(&name, &bytes)?;
    // Bundles keep their document id; each open copy needs its own.
    child.id = DocId::fresh();
    child.name = name;
    let index = s.add_document(child, None);
    // Admission may replace an ID already owned by another open document.
    let child_id = s.documents().get(index).ok_or(EngineError::NoDocument)?.doc.id;
    s.smart_links.retain(|l| l.child != child_id);
    s.smart_links.push(SmartLink { child: child_id, parent, layer: id, contents_id });
    Ok(json!({"document": index, "parentLayer": id.0}))
}

/// Writes an Edit Contents document back into all its parent instances (one undoable step in the
/// parent) and marks it saved. Returns false if `index` isn't an Edit Contents document.
pub fn commit_child(s: &mut Session, index: usize) -> Result<bool> {
    let Some(st) = s.docs.get(index) else { return Ok(false) };
    let child = st.doc.clone();
    let Some(link) = s.smart_links.iter().find(|l| l.child == child.id).copied() else { return Ok(false) };
    let parent = s.docs.iter().position(|d| d.doc.id == link.parent).ok_or_else(|| other("the smart object's document was closed"))?;
    let bytes = Arc::new(encode_source(&child)?);
    let prev = s.active;
    s.active = Some(parent);
    s.note_active_change();
    let r = s.edit("Edit Contents", |doc, _| {
        let ids = instances(doc, link.contents_id);
        if ids.is_empty() {
            return Err(other("the smart object's instances were removed"));
        }
        for id in ids {
            let sm = smart_mut(doc, id)?;
            let name = match &sm.source {
                SmartSource::Embedded { file_name, .. } => file_name.as_str(),
                SmartSource::Linked { .. } => child.name.as_str(),
            };
            let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
            sm.source = SmartSource::Embedded { file_name: format!("{stem}.pcraft"), bytes: bytes.clone() };
            detach_psd(doc.layer_mut(id).ok_or(EngineError::NoLayer(id))?);
            refresh_or_fail(doc, id)?;
        }
        Ok(())
    });
    s.active = prev;
    s.note_active_change();
    r?;
    if let Some(st) = s.docs.get_mut(index) {
        st.saved_revision = st.revision;
    }
    Ok(true)
}

/// Called before a document closes: an edited Edit Contents document updates its parent.
pub(crate) fn on_close(s: &mut Session, index: usize) {
    let Some(st) = s.docs.get(index) else { return };
    let id = st.doc.id;
    if st.is_dirty() && s.smart_links.iter().any(|l| l.child == id) {
        let _ = commit_child(s, index);
    }
    s.smart_links.retain(|l| l.child != id && l.parent != id);
    // Decoded sources are shared by content hash across documents, so the closing document's
    // can't be told apart: drop them all. Other documents decode again on their next re-render.
    if s.docs.get(index).is_some_and(|st| st.doc.walk().iter().any(|(_, _, l)| matches!(l.content, LayerContent::Smart(_)))) {
        super::purge_source_cache();
    }
}

pub(super) fn is_smart_child(s: &Session) -> std::result::Result<(), String> {
    let d = s.active().ok_or("no document open")?;
    if s.smart_links.iter().any(|l| l.child == d.doc.id) { Ok(()) } else { Err("this document isn't smart object contents".into()) }
}

pub(super) fn update_all(s: &mut Session) -> Result<Value> {
    let ids: Vec<LayerId> = s
        .active()
        .ok_or(EngineError::NoDocument)?
        .doc
        .walk()
        .into_iter()
        .filter(|(_, _, l)| matches!(&l.content, LayerContent::Smart(sm) if matches!(sm.source, SmartSource::Linked { .. })))
        .map(|(_, _, l)| l.id)
        .collect();
    let mut updated = Vec::new();
    s.edit("Update All Modified Content", |doc, _| {
        for id in &ids {
            if refresh(doc, *id)? {
                updated.push(id.0);
            }
        }
        Ok(())
    })?;
    Ok(json!({"updated": updated}))
}
