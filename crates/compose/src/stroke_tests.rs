//! Stroke effects on shape layers: the outline, the vector stroke's place, gradient frames.

use super::*;
use photocraft_color::{Color, ColorMode, PixelFormat, SampleType};
use photocraft_doc::vector::{Path, ShapeLayer, ShapeStroke, StrokeAlign, Subpath};
use photocraft_doc::{Effect, FxCommon, FxPaint, Gradient, GradientStyle, Layer, LayerContent, LayerMask, StrokeFx, StrokePosition};
use photocraft_geom::Size;

const E: f32 = 2.0 / 255.0;

fn close4(a: [f32; 4], b: [f32; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= E)
}

fn doc(w: u32, h: u32) -> Document {
    Document::with_background("t", Size::new(w, h), ColorMode::Rgb, SampleType::U8, Color::WHITE)
}

fn px(doc: &Document, x: i32, y: i32) -> [f32; 4] {
    render(doc, Rect::from_xywh(x, y, 1, 1)).px[0]
}

fn stroke(size: f32, position: StrokePosition, paint: FxPaint) -> Effect {
    Effect::Stroke(StrokeFx { common: FxCommon::new(BlendMode::Normal, 1.0), size, position, paint })
}

fn blue() -> FxPaint {
    FxPaint::Color(Color::rgb(0.0, 0.0, 1.0))
}

/// A shape layer over the square `x0..x1` with its Photoshop-style cached pixels.
fn shape(x0: f64, x1: f64, fill: Fill, stroke: Option<ShapeStroke>) -> Layer {
    let path = Path::new(vec![Subpath::polygon(&[(x0, x0), (x1, x0), (x1, x1), (x0, x1)])]);
    let mut sh = ShapeLayer { path, fill: Some(fill), stroke, live: None, cache: None, psd_raw: None };
    sh.cache = Some(photocraft_vector::render_shape(&sh, PixelFormat::RGBA8, Rect::new(0, 0, 40, 40)));
    Layer::new("shape", LayerContent::Shape(sh))
}

fn gradient_shape(width: u32, height: u32) -> ShapeLayer {
    let (w, h) = (f64::from(width), f64::from(height));
    ShapeLayer {
        path: Path::new(vec![Subpath::polygon(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)])]),
        fill: Some(Fill::gradient(vec![(0.0, Color::BLACK), (1.0, Color::WHITE)], 0.0, 1.0, GradientStyle::Linear, false)),
        stroke: Some(ShapeStroke { width: 4.0, ..ShapeStroke::default() }),
        live: None,
        cache: None,
        psd_raw: None,
    }
}

#[test]
fn split_stroked_gradient_with_effect_keeps_document_precision() {
    for depth in [SampleType::U8, SampleType::U16, SampleType::F32] {
        let mut d = Document::with_background("t", Size::new(4000, 40), ColorMode::Rgb, depth, Color::WHITE);
        let mut sh = gradient_shape(4000, 40);
        sh.cache = Some(photocraft_vector::render_shape(&sh, d.pixel_format(), d.bounds()));
        let mut layer = Layer::new("gradient shape", LayerContent::Shape(sh));
        layer.effects.items = vec![stroke(1.0, StrokePosition::Outside, blue())];
        d.layers.push(layer);
        let row = render(&d, Rect::from_xywh(0, 20, 4000, 1));
        let mut levels: Vec<u32> = row.px.iter().map(|p| (p[0] * 65535.0).round() as u32).collect();
        levels.sort_unstable();
        levels.dedup();
        if depth == SampleType::U8 {
            assert!(levels.len() <= 256, "8-bit control: {} levels", levels.len());
        } else {
            assert!(levels.len() > 3000, "{depth:?}: split gradient has only {} levels", levels.len());
        }
    }
}

#[test]
fn split_cache_keeps_same_shape_separate_at_each_document_depth() {
    let canvas = Rect::new(0, 0, 64, 8);
    let mut sh = gradient_shape(64, 8);
    // Keep the shape and cached tile identities identical while only the document depth changes.
    sh.cache = Some(photocraft_vector::render_shape(&sh, PixelFormat::RGBA32F, canvas));
    let mut layer = Layer::new("same shape", LayerContent::Shape(sh.clone()));
    layer.effects.items = vec![stroke(1.0, StrokePosition::Outside, blue())];
    for depth in [SampleType::U8, SampleType::U16, SampleType::F32, SampleType::U8] {
        let mut d = Document::with_background("t", Size::new(64, 8), ColorMode::Rgb, depth, Color::WHITE);
        d.layers.push(layer.clone());
        let expected = photocraft_vector::render_shape(&sh, PixelFormat::RGBA8.with_sample(depth), canvas).pixel(17, 4)[0];
        let actual = px(&d, 17, 4)[0];
        assert!((actual - expected).abs() < 1e-6, "{depth:?}: got {actual}, expected {expected}; split cache must include document depth");
    }
}

#[test]
fn outline_share_treats_stroke_and_shape_as_disjoint_areas() {
    // Outside the shape the stroke covers the pixel; inside it nothing.
    assert_eq!(effects::outline_share(0.0, 0.0), 1.0);
    assert_eq!(effects::outline_share(1.0, 0.3), 0.0);
    // Beneath an opaque edge pixel the result ends opaque: share + l (1 - share) = 1 - cov + l.
    let (cov, l) = (0.6, 0.6);
    let s = effects::outline_share(cov, l);
    assert!((s + l * (1.0 - s) - (1.0 - cov + l).min(1.0)).abs() < 1e-6);
    // A faded fill keeps the edge part of the stroke only.
    let s = effects::outline_share(0.6, 0.0);
    assert!((s - 0.4).abs() < 1e-6);
    assert_eq!(effects::outline_share(0.5, 1.0), 1.0);
    assert!(effects::outline_share(f32::NAN, 0.5).is_finite() || effects::outline_share(f32::NAN, 0.5).is_nan());
}

#[test]
fn filled_shape_stroke_follows_the_outline_where_the_fill_fades_out() {
    let mut d = doc(40, 40);
    let mut transparent = Color::rgb(1.0, 0.0, 0.0);
    transparent.alpha = 0.0;
    let fade = Fill::gradient(vec![(0.0, Color::rgb(1.0, 0.0, 0.0)), (1.0, transparent)], 0.0, 1.0, GradientStyle::Linear, false);
    let mut l = shape(10.0, 30.0, fade, None);
    l.effects.items = vec![stroke(3.0, StrokePosition::Outside, blue()), stroke(2.0, StrokePosition::Inside, FxPaint::Color(Color::rgb(0.0, 1.0, 0.0)))];
    d.layers.push(l);
    // The opaque side and the faded side both carry the outside stroke along the path.
    assert!(close4(px(&d, 8, 20), [0.0, 0.0, 1.0, 1.0]), "{:?}", px(&d, 8, 20));
    assert!(close4(px(&d, 31, 20), [0.0, 0.0, 1.0, 1.0]), "{:?}", px(&d, 31, 20));
    // ... and the inside stroke, even where the fill is transparent.
    assert!(close4(px(&d, 28, 20), [0.0, 1.0, 0.0, 1.0]), "{:?}", px(&d, 28, 20));
    // Between them the faded fill shows the background.
    assert!(px(&d, 25, 20)[0] > 0.9 && px(&d, 25, 20)[1] > 0.5, "{:?}", px(&d, 25, 20));
}

#[test]
fn outside_stroke_fills_the_rest_of_a_shapes_edge_pixel() {
    // The right edge at x = 30.5 covers half of column 30: an opaque fill and the stroke beneath
    // it end opaque there (no seam of backdrop between them).
    let mut d = Document::new("t", Size::new(40, 40), ColorMode::Rgb, SampleType::U8);
    let mut l = shape(10.0, 30.5, Fill::Solid(Color::rgb(1.0, 0.0, 0.0)), None);
    l.effects.items = vec![stroke(3.0, StrokePosition::Outside, blue())];
    d.layers.clear();
    d.layers.push(l);
    let p = px(&d, 30, 20);
    assert!((p[3] - 1.0).abs() <= E, "{p:?}");
    assert!((p[0] - 0.5).abs() <= 0.02 && (p[2] - 0.5).abs() <= 0.02, "{p:?}");
}

#[test]
fn vector_stroke_stays_above_interior_effects_and_below_the_stroke_effect() {
    let mut d = doc(40, 40);
    let vs = ShapeStroke { width: 4.0, align: StrokeAlign::Inside, paint: Fill::Solid(Color::rgb(0.0, 1.0, 0.0)), ..ShapeStroke::default() };
    let mut l = shape(10.0, 30.0, Fill::Solid(Color::rgb(1.0, 0.0, 0.0)), Some(vs));
    l.effects.items = vec![
        Effect::ColorOverlay { common: FxCommon::new(BlendMode::Normal, 1.0), color: Color::rgb(0.0, 0.0, 1.0) },
        stroke(1.0, StrokePosition::Inside, FxPaint::Color(Color::rgb(1.0, 1.0, 0.0))),
    ];
    d.layers.push(l);
    assert!(close4(px(&d, 10, 20), [1.0, 1.0, 0.0, 1.0]), "stroke effect on top: {:?}", px(&d, 10, 20));
    assert!(close4(px(&d, 12, 20), [0.0, 1.0, 0.0, 1.0]), "vector stroke over the overlay: {:?}", px(&d, 12, 20));
    assert!(close4(px(&d, 20, 20), [0.0, 0.0, 1.0, 1.0]), "overlay on the fill: {:?}", px(&d, 20, 20));
}

#[test]
fn a_stroked_shapes_mask_applies_to_fill_and_stroke_together() {
    let mut d = Document::new("t", Size::new(40, 40), ColorMode::Rgb, SampleType::U8);
    d.layers.clear();
    let vs = ShapeStroke { width: 4.0, align: StrokeAlign::Inside, paint: Fill::Solid(Color::rgb(0.0, 1.0, 0.0)), ..ShapeStroke::default() };
    let mut l = shape(10.0, 30.0, Fill::Solid(Color::rgb(1.0, 0.0, 0.0)), Some(vs));
    let mut m = LayerMask::reveal_all();
    m.surface = photocraft_raster::Surface::with_default(PixelFormat::GRAY8, &[0.5]);
    l.mask = Some(m);
    let mut clip = Layer::raster("clip", PixelFormat::RGBA8);
    clip.surface_mut().unwrap().fill_rect(Rect::new(0, 0, 40, 40), &[0.0, 0.0, 1.0, 1.0]);
    clip.clipped = true;
    d.layers.push(l.clone());
    d.layers.push(clip.clone());
    // On the vector stroke: the stroke's colour at the mask's 50 %, not 75 %.
    let p = px(&d, 12, 20);
    assert!(close4(p, [0.0, 1.0, 0.0, 0.5]), "{p:?}");
    // With effects too.
    l.effects.items = vec![Effect::ColorOverlay { common: FxCommon::new(BlendMode::Normal, 1.0), color: Color::rgb(1.0, 1.0, 1.0) }];
    d.layers = vec![l.clone()];
    let p = px(&d, 12, 20);
    assert!(close4(p, [0.0, 1.0, 0.0, 0.5]), "without clipped layers: {p:?}");
    d.layers = vec![l, clip];
    let p = px(&d, 12, 20);
    assert!(close4(p, [0.0, 1.0, 0.0, 0.5]), "{p:?}");
}

#[test]
fn gradient_stroke_spans_the_strokes_extent() {
    let mut d = doc(40, 40);
    let mut l = Layer::raster("sq", PixelFormat::RGBA8);
    l.surface_mut().unwrap().fill_rect(Rect::new(10, 10, 30, 30), &[1.0, 0.0, 0.0, 1.0]);
    let g = Gradient { stops: vec![(0.0, Color::BLACK), (1.0, Color::WHITE)], angle: 90.0, ..Gradient::default() };
    l.effects.items = vec![stroke(4.0, StrokePosition::Outside, FxPaint::Gradient(g))];
    d.layers.push(l);
    // The frame is the square grown by 3 px (7..33), laid out like every linear gradient
    // (fill_layout::gradient_layout); the layer's own bounds would give other values.
    let t_in = |frame: Rect, x: i32, y: i32| {
        let (a, s, o) = crate::fill_layout::gradient_layout(GradientStyle::Linear, 90.0, 1.0, (0.0, 0.0), frame);
        effects::gradient_t(GradientStyle::Linear, a, s, false, o, frame, x as f32 + 0.5, y as f32 + 0.5)
    };
    let (stroke_frame, layer_frame) = (Rect::new(7, 7, 33, 33), Rect::new(10, 10, 30, 30));
    for y in [7, 9, 31, 32] {
        let p = px(&d, 20, y);
        assert!((p[0] - t_in(stroke_frame, 20, y)).abs() <= 0.01, "row {y}: {p:?}");
    }
    assert!((t_in(stroke_frame, 20, 9) - t_in(layer_frame, 20, 9)).abs() > 0.05);
}

#[test]
fn several_strokes_stack_top_instance_first() {
    // Multiple instances: the first listed is on top; an outer wider stroke shows past it.
    let mut d = doc(60, 60);
    let mut l = Layer::raster("sq", PixelFormat::RGBA8);
    l.surface_mut().unwrap().fill_rect(Rect::new(20, 20, 40, 40), &[1.0, 0.0, 0.0, 1.0]);
    l.effects.items = vec![
        stroke(2.0, StrokePosition::Outside, FxPaint::Color(Color::rgb(0.0, 1.0, 0.0))),
        stroke(5.0, StrokePosition::Outside, blue()),
        stroke(2.0, StrokePosition::Inside, FxPaint::Color(Color::rgb(1.0, 1.0, 0.0))),
    ];
    d.layers.push(l);
    assert!(close4(px(&d, 19, 30), [0.0, 1.0, 0.0, 1.0]));
    assert!(close4(px(&d, 16, 30), [0.0, 0.0, 1.0, 1.0]));
    assert!(close4(px(&d, 20, 30), [1.0, 1.0, 0.0, 1.0]));
    assert!(close4(px(&d, 14, 30), [1.0; 4]));
}

#[test]
fn hairline_vector_stroke_keeps_the_files_edge_pixel_colour() {
    // The file's shape was snapped to whole pixels (Align Edges) with a 0.25 px stroke: its edge
    // pixels are opaque and mostly fill. Our path sits off the pixel grid, so our fill covers only
    // part of an edge pixel; fitting the stroke to the file's pixels must keep their colour there,
    // not paint the edge in the stroke's full colour. At every depth.
    for (sample, fmt, edge, fill) in [
        (SampleType::U8, PixelFormat::RGBA8, 0.86, 1.0),
        (SampleType::U16, PixelFormat::RGBA16, 0.86, 1.0),
        (SampleType::F32, PixelFormat::RGBA32F, 0.86, 1.0),
        (SampleType::F32, PixelFormat::RGBA32F, 2.0, 2.0),
    ] {
        let mut d = Document::with_background("t", Size::new(40, 40), ColorMode::Rgb, sample, Color::WHITE);
        let vs = ShapeStroke { width: 0.25, paint: Fill::Solid(Color::rgb(0.48, 0.48, 0.48)), ..ShapeStroke::default() };
        let path = Path::new(vec![Subpath::polygon(&[(10.3, 10.3), (29.7, 10.3), (29.7, 29.7), (10.3, 29.7)])]);
        let mut cache = photocraft_raster::Surface::new(fmt);
        cache.fill_rect(Rect::new(10, 10, 30, 30), &[edge, edge, edge, 1.0]);
        cache.fill_rect(Rect::new(11, 11, 29, 29), &[fill, fill, fill, 1.0]);
        let sh = ShapeLayer { path, fill: Some(Fill::Solid(Color::rgb(fill, fill, fill))), stroke: Some(vs), live: None, cache: Some(cache), psd_raw: None };
        let mut l = Layer::new("shape", LayerContent::Shape(sh));
        // Any layer effect sends the shape through the fill/stroke split.
        l.effects.items = vec![Effect::default_drop_shadow()];
        d.layers.push(l);
        for (x, y) in [(10, 20), (29, 20), (20, 10), (20, 29)] {
            let p = px(&d, x, y);
            assert!(close4(p, [edge, edge, edge, 1.0]), "{sample:?} edge pixel ({x}, {y}): {p:?}");
        }
        // The interior keeps the fill.
        assert!(close4(px(&d, 20, 20), [fill, fill, fill, 1.0]), "{sample:?} {:?}", px(&d, 20, 20));
    }
}

/// A 20 px square of (102, 102, 255) as type (`tysh`: its Photoshop data) or as plain pixels,
/// over a (204, 51, 51) backdrop.
fn square_on_red(text: bool, tysh: Option<&[u8]>, effect: Effect) -> Document {
    let mut d = Document::with_background("t", Size::new(60, 60), ColorMode::Rgb, SampleType::U8, Color::rgb(0.8, 0.2, 0.2));
    let mut l = Layer::raster("g", PixelFormat::RGBA8);
    l.surface_mut().unwrap().fill_rect(Rect::new(20, 20, 40, 40), &[0.4, 0.4, 1.0, 1.0]);
    if text {
        let t = photocraft_doc::TextLayer { cache: l.surface().cloned(), psd_raw: tysh.map(|b| std::sync::Arc::new(b.to_vec())), ..Default::default() };
        l = Layer::new("t", LayerContent::Text(t));
    }
    l.effects.items = vec![effect];
    d.layers.push(l);
    d
}

#[test]
fn type_layer_effects_mix_onto_the_backdrop_at_the_text_gamma() {
    // Photoshop mixes what a type layer's effects draw on the backdrop the way it mixes the type
    // itself. Measured in its own composite of real files (CC 2019): the half-covered outer edge
    // of a black Outside stroke around type over (102, 102, 255) is (44, 44, 121) at 32 %
    // backdrop, where the plain mix is (32, 32, 81); 98 % of 6,675 such pixels are within 2/255
    // of the gamma mix.
    let red = [0.8, 0.2, 0.2, 1.0];
    let gamma_mix = |ink: [f32; 3], a: f32, mode: BlendMode| psblend::composite_gamma(mode, red, [ink[0], ink[1], ink[2], a], 1.0, psblend::TEXT_GAMMA);
    let plain_mix = |ink: [f32; 3], a: f32, mode: BlendMode| psblend::composite(mode, red, [ink[0], ink[1], ink[2], a], 1.0);
    // A 6.5 px stroke ends on a half-covered pixel, 7 px out: x = 13.
    let st = || stroke(6.5, StrokePosition::Outside, FxPaint::Color(Color::BLACK));
    let (want, plain) = (gamma_mix([0.0; 3], 0.5, BlendMode::Normal), plain_mix([0.0; 3], 0.5, BlendMode::Normal));
    assert!((want[0] - plain[0]).abs() > 0.05, "the two mixes differ: {want:?} {plain:?}");
    let at = |d: &Document| px(d, 13, 30);
    assert!(close4(at(&square_on_red(true, None, st())), want), "live type: {:?} want {want:?}", at(&square_on_red(true, None, st())));
    assert!(close4(at(&square_on_red(false, None, st())), plain), "pixels: {:?} want {plain:?}", at(&square_on_red(false, None, st())));
    // Type Photoshop saved with substituted fonts is plain pixels to it.
    let substituted: &[u8] = b"\0\x01........\0\0\0\x0fsubstitutesUsedbool\x01";
    assert!(close4(at(&square_on_red(true, Some(substituted), st())), plain), "{:?}", at(&square_on_red(true, Some(substituted), st())));
    // Inside the stroke band and far outside it nothing changes.
    assert!(close4(px(&square_on_red(true, None, st()), 16, 30), [0.0, 0.0, 0.0, 1.0]));
    assert!(close4(px(&square_on_red(true, None, st()), 5, 30), red));

    // A hard-edged drop shadow at 50 %: the shadowed backdrop is the gamma mix for type.
    let shadow = || {
        Effect::DropShadow(photocraft_doc::Shadow {
            common: FxCommon::new(BlendMode::Multiply, 0.5),
            color: Color::BLACK,
            angle: 180.0,
            use_global_light: false,
            distance: 8.0,
            spread: 0.0,
            size: 0.0,
            contour: photocraft_doc::Contour::Linear,
            anti_alias: false,
            noise: 0.0,
            knocks_out: true,
        })
    };
    let (want, plain) = (gamma_mix([0.0; 3], 0.5, BlendMode::Multiply), plain_mix([0.0; 3], 0.5, BlendMode::Multiply));
    let shadowed = |d: &Document| [px(d, 44, 30), px(d, 15, 30)].into_iter().find(|p| (p[0] - red[0]).abs() > 0.05).unwrap_or(red);
    assert!(close4(shadowed(&square_on_red(true, None, shadow())), want), "type shadow: {:?} want {want:?}", shadowed(&square_on_red(true, None, shadow())));
    assert!(
        close4(shadowed(&square_on_red(false, None, shadow())), plain),
        "pixel shadow: {:?} want {plain:?}",
        shadowed(&square_on_red(false, None, shadow()))
    );
}
