use egui::Modifiers;
use photocraft_geom::{Affine, Rect};
use serde_json::json;

use super::*;

/// A family no machine has.
const MISSING: &str = "NoSuchFamily123";

fn text(app: &PhotocraftApp, id: LayerId) -> TextLayer {
    match &app.session.active().unwrap().doc.layer(id).unwrap().content {
        LayerContent::Text(t) => t.clone(),
        _ => panic!("not a type layer"),
    }
}

fn shown(app: &PhotocraftApp, id: LayerId) -> Rect {
    text(app, id).cache.unwrap().content_bounds()
}

fn steps(app: &PhotocraftApp) -> usize {
    app.session.active().unwrap().history.entries().len()
}

fn prompt(app: &PhotocraftApp) -> Option<&crate::state::Dialog> {
    app.ui.dialogs.iter().find(|d| owns(&d.fields))
}

/// A document with one type layer set in `family` (`None`: the default, installed one). With
/// `saved`, the layer holds pixels our engine did not draw, as a PSD's type does until it is
/// edited: the same text drawn somewhere else. Returns the layer and the bounds of what it shows.
fn app_with_type(family: Option<&str>, saved: bool) -> (PhotocraftApp, LayerId, Rect) {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
    app.run("file.new", json!({"width": 1600, "height": 900})).unwrap();
    app.sync_views();
    app.ui.tool = crate::state::Tool::Type;
    let mut p = json!({"text": "HOHOHO", "size": 120, "x": 300, "y": 420});
    if let Some(family) = family {
        p["font"] = json!(family);
    }
    let id = LayerId(app.run("type.create", p).unwrap()["layer"].as_u64().unwrap());
    if saved {
        let moved = {
            let mut t = text(&app, id);
            t.transform = Affine::translate(330.0, 380.0);
            photocraft_text::shared().lock().unwrap().render(&t, 72.0, photocraft_color::PixelFormat::RGBA8).1.surface
        };
        let st = app.session.active_mut().unwrap();
        let mut doc = (*st.doc).clone();
        if let Some(LayerContent::Text(t)) = doc.layer_mut(id).map(|l| &mut l.content) {
            t.cache = Some(moved);
        }
        st.doc = std::sync::Arc::new(doc);
        st.revision += 1;
    }
    // As after opening the file: nothing has been said about its fonts yet.
    app.ui.notices.clear();
    app.ui.missing_font_noticed.clear();
    let bounds = shown(&app, id);
    (app, id, bounds)
}

fn centre(r: Rect) -> (f64, f64) {
    (f64::from(r.x0 + r.x1) / 2.0, f64::from(r.y0 + r.y1) / 2.0)
}

#[test]
fn editing_saved_type_in_a_missing_font_asks_first_and_cancel_keeps_the_pixels() {
    let (mut app, id, saved) = app_with_type(Some(MISSING), true);
    let before = steps(&app);
    let (x, y) = centre(saved);
    assert!(crate::type_tool::pointer_down(&mut app, x, y, Modifiers::NONE), "the click is used up");
    assert!(app.ui.text_edit.is_none(), "the edit waits for the answer");
    let d = prompt(&app).expect("the prompt is open");
    assert_eq!(d.fields["fonts"], json!([MISSING]));
    assert_eq!(d.fields["layer"], json!(id.0));
    assert!(d.fields["message"].as_str().unwrap().contains(MISSING), "{}", d.fields["message"]);
    assert_eq!(crate::dialogs::title(d), "Missing Fonts");
    let dialog = d.id;
    assert_eq!(shown(&app, id), saved, "nothing is redrawn before the answer");
    assert_eq!(steps(&app), before);
    // A second click on the text is the same question, not another dialog.
    assert!(crate::type_tool::pointer_down(&mut app, x, y, Modifiers::NONE));
    assert_eq!(app.ui.dialogs.len(), 1);
    crate::dialogs::cancel(&mut app, dialog).unwrap();
    assert!(app.ui.text_edit.is_none());
    assert_eq!(shown(&app, id), saved, "Cancel keeps the text as it was saved");
    assert_eq!(steps(&app), before);
    assert!(app.ui.notices.is_empty());
}

#[test]
fn ok_starts_the_edit_in_the_substitute_face_as_one_step_that_cancel_undoes() {
    // From the layer's thumbnail (Edit Type): all the text is selected.
    let (mut app, id, saved) = app_with_type(Some(MISSING), true);
    let before = steps(&app);
    let r = crate::menus::invoke(&mut app, &egui::Context::default(), "type.editText", json!({})).unwrap();
    let dialog = r["dialog"].as_u64().expect("Edit Type answers with the prompt");
    assert!(app.ui.text_edit.is_none());
    let done = crate::dialogs::confirm(&mut app, dialog).unwrap();
    assert_eq!(done, json!({"editing": id.0, "substituted": [MISSING]}));
    let ed = app.ui.text_edit.clone().expect("the edit started");
    assert_eq!((ed.layer, ed.anchor, ed.caret), (id.0, 0, 6));
    assert_ne!(shown(&app, id), saved, "redrawn by our engine, where the caret is");
    assert_eq!(steps(&app), before + 1);
    assert!(app.ui.notices.is_empty(), "the user just agreed to it: no notice on top");
    crate::type_tool::cancel(&mut app);
    assert_eq!(shown(&app, id), saved, "cancelling the edit brings the saved pixels back");
    assert_eq!(steps(&app), before);

    // From a click on the text: the caret goes where the click was.
    let (mut app, id, saved) = app_with_type(Some(MISSING), true);
    let (x, y) = centre(saved);
    assert!(crate::type_tool::pointer_down(&mut app, x, y, Modifiers::NONE));
    let d = prompt(&app).unwrap();
    assert_eq!(d.fields["at"], json!([x, y]));
    let dialog = d.id;
    crate::dialogs::confirm(&mut app, dialog).unwrap();
    let ed = app.ui.text_edit.clone().expect("the edit started");
    assert_eq!(ed.layer, id.0);
    assert_eq!(ed.anchor, ed.caret, "a caret, not a selection");
    assert!(app.ui.dialogs.is_empty());
}

#[test]
fn a_changed_document_refuses_the_answer() {
    let (mut app, id, saved) = app_with_type(Some(MISSING), true);
    let (x, y) = centre(saved);
    assert!(crate::type_tool::pointer_down(&mut app, x, y, Modifiers::NONE));
    let dialog = prompt(&app).unwrap().id;
    app.run("layer.delete", json!({"layer": id.0})).unwrap();
    assert!(crate::dialogs::confirm(&mut app, dialog).is_err());
    assert!(app.ui.text_edit.is_none());
}

#[test]
fn installed_fonts_and_text_already_redrawn_do_not_ask() {
    // Saved pixels in an installed family: the edit redraws the text faithfully, as before.
    let (mut app, id, saved) = app_with_type(None, true);
    let (x, y) = centre(saved);
    assert!(crate::type_tool::pointer_down(&mut app, x, y, Modifiers::NONE));
    assert!(prompt(&app).is_none());
    assert_eq!(app.ui.text_edit.as_ref().map(|e| e.layer), Some(id.0));

    // A missing family whose text our engine has already drawn: what is shown is the substitute.
    let (mut app, id, ours) = app_with_type(Some(MISSING), false);
    let (x, y) = centre(ours);
    assert!(crate::type_tool::pointer_down(&mut app, x, y, Modifiers::NONE));
    assert!(prompt(&app).is_none());
    assert_eq!(app.ui.text_edit.as_ref().map(|e| e.layer), Some(id.0));
}

#[test]
fn a_type_command_that_substitutes_a_font_says_so_once_per_document_and_family() {
    let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
    app.run("file.new", json!({"width": 400, "height": 200})).unwrap();
    let id = app.run("type.create", json!({"text": "Hello", "size": 24, "x": 20, "y": 60, "font": MISSING})).unwrap()["layer"].as_u64().unwrap();
    assert_eq!(app.ui.notices.len(), 1);
    assert_eq!(app.ui.notices[0].title, "Text was drawn in a substitute font");
    assert_eq!(app.ui.notices[0].lines[0], MISSING);
    assert!(!app.ui.notices[0].error);
    // Every keystroke reports the family again; the notice does not come back.
    app.run("type.edit", json!({"layer": id, "replace": {"start": 5, "end": 5, "text": "!"}})).unwrap();
    assert_eq!(app.ui.notices.len(), 1);
    app.run("type.setStyle", json!({"layer": id, "font": "StillNoSuchFamily456"})).unwrap();
    assert_eq!(app.ui.notices.len(), 2, "another family is news");
    assert_eq!(app.ui.notices[1].lines[0], "StillNoSuchFamily456");
    // The same family in another document is news there.
    app.run("file.new", json!({"width": 400, "height": 200})).unwrap();
    app.ui.notices.clear();
    app.run("type.create", json!({"text": "Hello", "size": 24, "x": 20, "y": 60, "font": MISSING})).unwrap();
    assert_eq!(app.ui.notices.len(), 1);
    // An installed family says nothing.
    app.ui.notices.clear();
    app.run("type.create", json!({"text": "Hello", "size": 24, "x": 20, "y": 120})).unwrap();
    assert!(app.ui.notices.is_empty());
}

#[test]
fn opening_a_document_names_its_missing_fonts() {
    let (mut app, _, _) = app_with_type(Some(MISSING), true);
    document_opened(&mut app);
    assert!(app.ui.missing_font_pending.is_empty());
    assert_eq!(app.ui.notices.len(), 1);
    assert_eq!(app.ui.notices[0].title, "Fonts used in this document are not installed");
    assert_eq!(app.ui.notices[0].lines[0], MISSING);
    // It says what the document needs; the first edit still asks.
    assert!(app.ui.missing_font_noticed.is_empty());

    let (mut app, _, _) = app_with_type(None, true);
    document_opened(&mut app);
    assert!(app.ui.notices.is_empty(), "every font is installed: nothing to say");
}

#[test]
fn the_layers_panel_marks_type_whose_font_is_missing() {
    let (mut app, id, _) = app_with_type(Some(MISSING), true);
    let doc = app.session.active().unwrap().doc.clone();
    assert!(layer_missing(&mut app, &doc, id));
    let background = doc.layers.first().unwrap().id;
    assert!(!layer_missing(&mut app, &doc, background));
    // Choosing an installed family clears the mark (a new document revision).
    app.run("type.setStyle", json!({"layer": id.0, "font": photocraft_text::fonts::DEFAULT_FAMILY})).unwrap();
    let doc = app.session.active().unwrap().doc.clone();
    assert!(!layer_missing(&mut app, &doc, id));
}

/// The whole window laid out and painted frame by frame (no renderer): the notice from opening,
/// the Layers panel with its caution mark, the prompt with its question, then the edit itself.
#[test]
fn the_notice_the_layer_mark_and_the_prompt_are_drawn_in_the_window() {
    use egui_kittest::kittest::Queryable;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).with_max_steps(64).build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let (mut app, _, _) = app_with_type(Some(MISSING), true);
        document_opened(&mut app);
        app
    });
    h.run_steps(4);
    assert!(h.query_by_label_contains("Fonts used in this document are not installed").is_some(), "the notice is on screen");
    let id = h.state().session.active().unwrap().active_layer.unwrap();
    let marked = h.state().missing_font_layers.as_ref().map(|(_, ids)| ids.clone());
    assert_eq!(marked, Some(vec![id.0]), "the Layers panel drew the type layer's thumbnail with the mark");

    let ctx = h.ctx.clone();
    let dialog = crate::menus::invoke(h.state_mut(), &ctx, "type.editText", json!({})).unwrap()["dialog"].as_u64().unwrap();
    h.run_steps(4);
    assert!(h.query_by_label_contains("Editing it will redraw the text in a substitute font").is_some(), "the prompt shows its question");
    assert!(h.state().ui.text_edit.is_none());

    crate::dialogs::confirm(h.state_mut(), dialog).unwrap();
    h.run_steps(4);
    assert!(prompt(h.state()).is_none());
    assert_eq!(h.state().ui.text_edit.as_ref().map(|e| e.layer), Some(id.0), "the edit runs, with its caret and frame drawn");
}

#[test]
fn the_strings_are_translated_in_every_complete_catalog() {
    for lang in crate::i18n::Lang::all().filter(|l| l.complete_menus()) {
        for key in [
            "Missing Fonts",
            "This text uses fonts that are not installed: {fonts}. Editing it will redraw the text in a substitute font. Continue?",
            "Text was drawn in a substitute font",
            "Choose an installed font for the text, or undo to bring back the text as it was.",
            "Fonts used in this document are not installed",
            "Type layers look as they were saved until their text is edited.",
        ] {
            assert_ne!(crate::i18n::tr(lang, key), key, "{}: {key}", lang.code());
        }
    }
}
