//! Missing fonts in the window: Photoshop's "font substitution will occur" question, a caution
//! mark on the layer and notices, so text is never restyled without the user knowing.
//!
//! A PSD's type layer keeps the pixels the file was saved with until its text is edited, so a
//! document whose fonts aren't installed opens looking right. The first edit redraws the layer
//! with our type engine, in a substitute face when the family is missing:
//!
//! - opening such a document posts a notice naming the families ([`document_opened`]);
//! - starting to edit such a layer asks first ([`intercept`]): Cancel leaves the saved pixels
//!   alone, OK starts the edit. The prompt is an ordinary dialog in the UI state (`ui.inspect`
//!   shows it, `ui.dialog.confirm` and `ui.dialog.cancel` answer it);
//! - a `type.*` command that reports a missing family any other way (the options bar, Update All
//!   Text Layers) posts a notice, once per document and family ([`command_ran`]);
//! - the Layers panel marks the layer's thumbnail ([`layer_missing`]).

use photocraft_doc::{Document, LayerContent, LayerId, TextLayer};
use photocraft_engine::type_extra_cmds::{missing_fonts, missing_fonts_in_layer};
use serde_json::{Map, Value, json};

use crate::PhotocraftApp;
use crate::state::DialogKind;

const MARK: &str = "__missingFont";

/// Is this dialog the prompt?
pub fn owns(fields: &Map<String, Value>) -> bool {
    fields.contains_key(MARK)
}

/// `fonts` as shown to the user: `Trajan Pro 3, Montserrat`.
fn listed(fonts: &[String]) -> String {
    fonts.join(", ")
}

/// Remember that the user has been told about `fonts` in document `doc`; returns the ones that
/// were news.
fn mark_noticed(app: &mut PhotocraftApp, doc: u64, fonts: &[String]) -> Vec<String> {
    let mut fresh = Vec::new();
    for f in fonts {
        let key = (doc, f.to_lowercase());
        if !app.ui.missing_font_noticed.contains(&key) {
            app.ui.missing_font_noticed.push(key);
            fresh.push(f.clone());
        }
    }
    fresh
}

/// An edit of type layer `id` is about to redraw the pixels the file was saved with (`t` is its
/// text). When a family it uses isn't installed, open the prompt and return its dialog id: the
/// edit waits for the answer. `at` is the document point of the click that started the edit
/// (`None`: the layer's thumbnail or the menu, which select all the text).
pub fn intercept(app: &mut PhotocraftApp, id: LayerId, t: &TextLayer, at: Option<[f64; 2]>) -> Option<u64> {
    let fonts = missing_fonts_in_layer(t);
    if fonts.is_empty() {
        return None;
    }
    let doc = app.session.active()?.doc.id.0;
    // One question at a time: a second click on the same text is the same question.
    if let Some(open) = app.ui.dialogs.iter().find(|d| owns(&d.fields)) {
        return Some(open.id);
    }
    let message = crate::i18n::fmt(
        tl!("This text uses fonts that are not installed: {fonts}. Editing it will redraw the text in a substitute font. Continue?"),
        &[("fonts", &listed(&fonts))],
    );
    let mut f = Map::new();
    f.insert(MARK.into(), json!(true));
    f.insert("__label".into(), json!("Missing Fonts"));
    f.insert("message".into(), json!(message));
    f.insert("fonts".into(), json!(fonts));
    f.insert("document".into(), json!(doc));
    f.insert("layer".into(), json!(id.0));
    if let Some(at) = at {
        f.insert("at".into(), json!(at));
    }
    Some(app.ui.open_dialog(DialogKind::Command, f))
}

/// The prompt's body: the question, with a caution mark.
pub fn body(ui: &mut egui::Ui, f: &Map<String, Value>) {
    let t = crate::theme::Tokens::get(ui.ctx());
    let text = f.get("message").and_then(Value::as_str).unwrap_or_default();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let (r, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
        crate::icons::paint(ui, r, "triangle-alert", 30.0, t.warning);
        ui.add(egui::Label::new(egui::RichText::new(text).color(t.text).size(13.0)).wrap());
    });
}

/// OK: start the edit the prompt held back. The substitute face is the user's choice now, so
/// the notice for these families is not shown on top of it.
pub fn confirm(app: &mut PhotocraftApp, f: &Map<String, Value>) -> Result<Value, String> {
    let layer = LayerId(f.get("layer").and_then(Value::as_u64).ok_or("the prompt has no layer")?);
    let st = app.session.active().ok_or("no document open")?;
    let doc = st.doc.id.0;
    if f.get("document").and_then(Value::as_u64) != Some(doc) || !matches!(st.doc.layer(layer).map(|l| &l.content), Some(LayerContent::Text(_))) {
        return Err("the document changed; start the edit again".into());
    }
    let fonts: Vec<String> =
        f.get("fonts").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let at = f.get("at").and_then(Value::as_array).and_then(|a| Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?])).filter(|p| p.iter().all(|v| v.is_finite()));
    mark_noticed(app, doc, &fonts);
    crate::type_tool::edit_confirmed(app, layer, at)?;
    Ok(json!({"editing": layer.0, "substituted": fonts}))
}

/// The family named by one of the engine's missing-font warnings
/// (`font 'X' is not installed; text may render using a fallback face`).
fn family_of(warning: &str) -> Option<&str> {
    warning.strip_prefix("font '")?.split_once("' is not installed").map(|(family, _)| family)
}

/// A command ran: when a `type.*` command's result warns that a family isn't installed, say so
/// in a notice, once per document and family (typing reports it on every keystroke).
pub fn command_ran(app: &mut PhotocraftApp, id: &str, result: &Value) {
    if !id.starts_with("type.") {
        return;
    }
    let Some(warnings) = result.get("warnings").and_then(Value::as_array) else { return };
    let fonts: Vec<String> = warnings.iter().filter_map(Value::as_str).filter_map(family_of).map(str::to_string).collect();
    let Some(doc) = app.session.active().map(|st| st.doc.id.0) else { return };
    let fresh = mark_noticed(app, doc, &fonts);
    if fresh.is_empty() {
        return;
    }
    let lines = vec![listed(&fresh), "Choose an installed font for the text, or undo to bring back the text as it was.".to_owned()];
    crate::notices::post(app, "Text was drawn in a substitute font", lines, false, None);
}

/// A document was opened (it is the active one): its missing fonts get a notice as soon as the
/// font database is ready.
pub fn document_opened(app: &mut PhotocraftApp) {
    if let Some(doc) = app.session.active().map(|st| st.doc.id.0)
        && !app.ui.missing_font_pending.contains(&doc)
    {
        app.ui.missing_font_pending.push(doc);
    }
    tick(app);
}

/// Post the notices [`document_opened`] queued. Called every frame; it never blocks on the
/// start-up font scan (a document opened at launch waits here until the scan is done).
pub fn tick(app: &mut PhotocraftApp) {
    if app.ui.missing_font_pending.is_empty() || photocraft_text::try_shared().is_none() {
        return;
    }
    for doc in std::mem::take(&mut app.ui.missing_font_pending) {
        let Some(fonts) = app.session.documents().iter().find(|st| st.doc.id.0 == doc).map(|st| missing_fonts(&st.doc)) else { continue };
        // Not marked as noticed: this says what the document needs, not that text was redrawn.
        if !fonts.is_empty() {
            let lines = vec![listed(&fonts), "Type layers look as they were saved until their text is edited.".to_owned()];
            crate::notices::post(app, "Fonts used in this document are not installed", lines, false, None);
        }
    }
}

/// Does type layer `layer` of the active document `doc` use a family that isn't installed? For
/// the Layers panel's caution mark: computed once per document revision, and `false` while the
/// installed fonts are still being scanned.
pub fn layer_missing(app: &mut PhotocraftApp, doc: &Document, layer: LayerId) -> bool {
    let Some(revision) = app.session.active().filter(|st| st.doc.id == doc.id).map(|st| st.revision) else { return false };
    let key = (doc.id.0, revision, photocraft_text::fonts::generation());
    if let Some((k, ids)) = &app.missing_font_layers
        && *k == key
    {
        return ids.contains(&layer.0);
    }
    if photocraft_text::try_shared().is_none() {
        return false;
    }
    let ids: Vec<u64> = doc
        .walk()
        .into_iter()
        .filter_map(|(_, _, l)| match &l.content {
            LayerContent::Text(t) if !missing_fonts_in_layer(t).is_empty() => Some(l.id.0),
            _ => None,
        })
        .collect();
    let found = ids.contains(&layer.0);
    app.missing_font_layers = Some((key, ids));
    found
}

#[cfg(test)]
#[path = "missing_font_prompt_tests.rs"]
mod tests;
