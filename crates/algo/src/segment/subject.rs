//! Select Subject (classical, heuristic).
//!
//! Photoshop uses a learned model; we approximate it with salient-object detection feeding a
//! GrabCut:
//!
//! 1. SLIC superpixels on a working-resolution copy.
//! 2. Per-superpixel saliency from "Saliency Optimization from Robust Background Detection"
//!    (W. Zhu, S. Liang, Y. Wei and J. Sun, CVPR 2014): boundary connectivity
//!    `BndCon(p) = Len_bnd(p) / sqrt(Area(p))` over geodesic colour distances gives a background
//!    probability; background-weighted contrast gives foreground cues; a least-squares
//!    optimisation with smoothness between neighbours combines them.
//! 3. Blended with frequency-tuned saliency (R. Achanta, S. Hemami, F. Estrada and
//!    S. Süsstrunk, "Frequency-tuned Salient Region Detection", CVPR 2009): distance of the
//!    lightly blurred CIELAB image from its mean colour.
//! 4. Otsu's threshold on the saliency map seeds a GrabCut trimap (low-saliency border pixels
//!    are definite background); the result is cleaned (small islands dropped, small holes
//!    filled) and re-cut at full resolution.
//! 5. The bottom side of the picture is not read as background, in the boundary connectivity of
//!    step 2 or in the trimap of step 4: the subject of a still often leaves the frame through
//!    it, while a floor or a table reaches a left or right side too.
//! 6. When the selection so far reads as a face (skin tone in three tenths of it or more), a
//!    second GrabCut is offered what lies below it, spreading at 45° to at most one face-width
//!    on each side, as probable foreground: a head-and-shoulders still is the person, and
//!    clothes share no colour with a face. The cut still decides by colour.
//!
//! Works well for a distinct object on a comparatively plain background, and for one person
//! from the chest up; it is not a semantic segmenter.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use photocraft_geom::Rect;

use super::grabcut::{self, BG, PR_BG, PR_FG};
use super::slic::{rgb_to_lab, slic_lab};
use super::{Region, Sampler};

/// Working-resolution budget (pixels).
pub const WORK_PX: usize = 110_000;
const SIGMA_CLR: f32 = 10.0;
const SIGMA_SPA: f32 = 0.25;

/// Every side of the picture ([`SubjectTuning::bg_sides`]).
pub const ALL_SIDES: u8 = 0b1111;
/// Every side but the bottom one.
pub const NOT_BOTTOM: u8 = 0b1011;

/// The head-and-shoulders completion of [`select_subject_tuned`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Portrait {
    /// One cut, as the module docs describe.
    Off,
    /// A second cut in which the hair around the first selection and the shoulders below it are
    /// probable foreground.
    Probable,
    /// As `Probable`, with everything straight below the first selection definite foreground.
    Core,
}

/// Knobs of [`select_subject_tuned`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubjectTuning {
    /// Picture sides read as background evidence: bit 0 top, 1 right, 2 bottom, 3 left.
    pub bg_sides: u8,
    /// The head-and-shoulders completion.
    pub portrait: Portrait,
    /// Least share of skin-toned pixels in the first selection for the completion to read it
    /// as a face.
    pub skin_gate: f32,
    /// How far the shoulders reach sideways beyond the first selection, in widths of the face
    /// (the widest row of the first selection).
    pub shoulder: f32,
    /// How far the hair reaches beyond the first selection, in widths of the face.
    pub hair: f32,
}

impl SubjectTuning {
    /// What [`select_subject`] does.
    pub const SHIPPED: Self = Self { bg_sides: NOT_BOTTOM, portrait: Portrait::Probable, skin_gate: 0.3, shoulder: 1.0, hair: 0.0 };
}

impl Default for SubjectTuning {
    fn default() -> Self {
        Self::SHIPPED
    }
}

fn on_side(x: usize, y: usize, w: usize, h: usize, sides: u8) -> bool {
    (y == 0 && sides & 1 != 0) || (x + 1 == w && sides & 2 != 0) || (y + 1 == h && sides & 4 != 0) || (x == 0 && sides & 8 != 0)
}

/// Selects the most salient object on the canvas.
pub fn select_subject(sampler: &dyn Sampler, canvas: Rect) -> Option<Region> {
    select_subject_tuned(sampler, canvas, &SubjectTuning::SHIPPED)
}

/// [`select_subject`] with its knobs exposed (measurements, tests).
pub fn select_subject_tuned(sampler: &dyn Sampler, canvas: Rect, tune: &SubjectTuning) -> Option<Region> {
    if canvas.width() < 4 || canvas.height() < 4 {
        return None;
    }
    let step = super::scale_for(canvas, WORK_PX);
    let img = sampler.rgb_scaled(canvas, step);
    let (w, h) = (img.w, img.h);
    let sal = saliency_sides(&img, tune.bg_sides);
    let t = otsu(&sal).clamp(0.15, 0.85);
    let mut trimap = first_trimap(&sal, t, w, h, tune.bg_sides);
    if !trimap.contains(&PR_FG) {
        return None;
    }
    let (mut fg, mut bg) = grabcut::grabcut(&img, &mut trimap, 6)?;
    let low: Vec<bool> = trimap.iter().map(|v| *v == PR_FG || *v == grabcut::FG).collect();
    let mut low = super::clean_mask(&low, w, h, 0.2, 0.05);
    if !low.iter().any(|v| *v) {
        return None;
    }
    if tune.portrait != Portrait::Off
        && let Some(mut second) = portrait_trimap(&img, &sal, t, &low, tune)
        && let Some((f2, b2)) = grabcut::grabcut(&img, &mut second, 6)
    {
        let grown: Vec<bool> = second.iter().map(|v| *v == PR_FG || *v == grabcut::FG).collect();
        let grown = super::clean_mask(&grown, w, h, 0.2, 0.05);
        let (before, after) = (low.iter().filter(|v| **v).count(), grown.iter().filter(|v| **v).count());
        // The completion adds to a selection; a second cut that lost part of the first is dropped.
        if after >= before {
            (low, fg, bg) = (grown, f2, b2);
        }
    }
    super::finish_region(sampler, canvas, step, &low, w, h, Some((&fg, &bg)))
}

/// The trimap of the first cut: salient pixels probable foreground, the chosen sides and the
/// least salient pixels definite background.
fn first_trimap(sal: &[f32], t: f32, w: usize, h: usize, sides: u8) -> Vec<u8> {
    let mut trimap = vec![PR_BG; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            trimap[i] = if sal[i] >= t {
                PR_FG
            } else if on_side(x, y, w, h, sides) || sal[i] < 0.4 * t {
                BG
            } else {
                PR_BG
            };
        }
    }
    trimap
}

/// Skin tone by chromaticity (a face lit warm or neutral; not a face under coloured light).
fn is_skin(p: [f32; 3]) -> bool {
    let [r, g, b] = p;
    let sum = r + g + b;
    if sum < 0.25 || r <= g || r <= b || r - g.min(b) < 0.06 {
        return false;
    }
    let (rn, gn) = (r / sum, g / sum);
    (0.36..=0.56).contains(&rn) && (0.24..=0.37).contains(&gn)
}

/// The trimap of the head-and-shoulders completion, or `None` when the first selection `low`
/// does not read as a face (too little skin tone in it). Two bands around the first selection
/// become probable foreground: everything within `hair` face-widths of it (the hair), and
/// everything below it, spreading one pixel sideways per row down to at most `shoulder`
/// face-widths (the shoulders, down to the bottom of the picture, which this pass does not read
/// as background there). The face's width is the widest row of the first selection. The first
/// selection itself stays probable foreground, so the second cut may still correct it.
fn portrait_trimap(img: &super::RgbImage, sal: &[f32], t: f32, low: &[bool], tune: &SubjectTuning) -> Option<Vec<u8>> {
    let (w, h) = (img.w, img.h);
    let area = low.iter().filter(|v| **v).count();
    let skin = low.iter().zip(&img.px).filter(|(l, p)| **l && is_skin(**p)).count();
    if area == 0 || (skin as f32) < tune.skin_gate * area as f32 {
        return None;
    }
    let widest = (0..h).map(|y| low[y * w..(y + 1) * w].iter().filter(|v| **v).count()).max().unwrap_or(0) as f32;
    let (reach_hair, reach_body) = (tune.hair * widest, tune.shoulder * widest);
    const FAR: f32 = 1.0e9;
    const DIAG: f32 = std::f32::consts::SQRT_2;
    // Distance from the first selection (two-pass chamfer).
    let mut dist: Vec<f32> = low.iter().map(|l| if *l { 0.0 } else { FAR }).collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut d = dist[i];
            if x > 0 {
                d = d.min(dist[i - 1] + 1.0);
            }
            if y > 0 {
                d = d.min(dist[i - w] + 1.0);
                if x > 0 {
                    d = d.min(dist[i - w - 1] + DIAG);
                }
                if x + 1 < w {
                    d = d.min(dist[i - w + 1] + DIAG);
                }
            }
            dist[i] = d;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            let mut d = dist[i];
            if x + 1 < w {
                d = d.min(dist[i + 1] + 1.0);
            }
            if y + 1 < h {
                d = d.min(dist[i + w] + 1.0);
                if x + 1 < w {
                    d = d.min(dist[i + w + 1] + DIAG);
                }
                if x > 0 {
                    d = d.min(dist[i + w - 1] + DIAG);
                }
            }
            dist[i] = d;
        }
    }
    // Sideways steps from the first selection on the way down: 0 straight below it.
    let mut side = vec![FAR; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if low[i] {
                side[i] = 0.0;
            } else if y > 0 {
                let up = i - w;
                let mut s = side[up];
                if x > 0 {
                    s = s.min(side[up - 1] + 1.0);
                }
                if x + 1 < w {
                    s = s.min(side[up + 1] + 1.0);
                }
                side[i] = s;
            }
        }
    }
    let mut trimap = first_trimap(sal, t, w, h, tune.bg_sides);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if low[i] {
                trimap[i] = PR_FG;
            } else if tune.portrait == Portrait::Core && side[i] == 0.0 {
                trimap[i] = grabcut::FG;
            } else if (dist[i] <= reach_hair || side[i] <= reach_body) && !on_side(x, y, w, h, tune.bg_sides & NOT_BOTTOM) {
                // The other three sides stay background.
                trimap[i] = PR_FG;
            }
        }
    }
    Some(trimap)
}

/// Per-pixel saliency in `0..=1` (see the module docs).
pub fn saliency(img: &super::RgbImage) -> Vec<f32> {
    saliency_sides(img, ALL_SIDES)
}

/// [`saliency`] with only the given sides of the picture read as its boundary
/// ([`SubjectTuning::bg_sides`]).
pub fn saliency_sides(img: &super::RgbImage, sides: u8) -> Vec<f32> {
    let (w, h) = (img.w, img.h);
    let lab: Vec<[f32; 3]> = img.px.iter().map(|p| rgb_to_lab(*p)).collect();
    let target = ((w * h) / 150).clamp(16, 400);
    let sp = slic_lab(&lab, w, h, target, 10.0, 8);
    let n = sp.count;
    // Region statistics.
    let mut mean = vec![[0.0f64; 3]; n];
    let mut pos = vec![[0.0f64; 2]; n];
    let mut cnt = vec![0.0f64; n];
    let mut on_border = vec![false; n];
    for y in 0..h {
        for x in 0..w {
            let l = sp.labels[y * w + x] as usize;
            let c = lab[y * w + x];
            for k in 0..3 {
                mean[l][k] += c[k] as f64;
            }
            pos[l][0] += x as f64 / w as f64;
            pos[l][1] += y as f64 / h as f64;
            cnt[l] += 1.0;
            if on_side(x, y, w, h, sides) {
                on_border[l] = true;
            }
        }
    }
    let mean: Vec<[f32; 3]> = mean.iter().zip(&cnt).map(|(m, c)| m.map(|v| (v / c.max(1.0)) as f32)).collect();
    let pos: Vec<[f32; 2]> = pos.iter().zip(&cnt).map(|(p, c)| p.map(|v| (v / c.max(1.0)) as f32)).collect();
    let dapp = |a: usize, b: usize| super::d2(mean[a], mean[b]).sqrt();
    // Adjacency.
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut link = |a: usize, b: usize| {
        if a != b && !adj[a].contains(&b) {
            adj[a].push(b);
            adj[b].push(a);
        }
    };
    for y in 0..h {
        for x in 0..w {
            let l = sp.labels[y * w + x] as usize;
            if x + 1 < w {
                link(l, sp.labels[y * w + x + 1] as usize);
            }
            if y + 1 < h {
                link(l, sp.labels[(y + 1) * w + x] as usize);
            }
        }
    }
    // Boundary connectivity from all-pairs geodesic distances (Dijkstra per region).
    let mut wbg = vec![0.0f32; n];
    let mut geo = vec![f32::INFINITY; n];
    for (p, wb) in wbg.iter_mut().enumerate() {
        dijkstra(p, &adj, &dapp, &mut geo);
        let (mut area, mut len) = (0.0f32, 0.0f32);
        for q in 0..n {
            let s = (-geo[q] * geo[q] / (2.0 * SIGMA_CLR * SIGMA_CLR)).exp();
            area += s;
            if on_border[q] {
                len += s;
            }
        }
        let bnd = len / area.max(1e-6).sqrt();
        *wb = 1.0 - (-bnd * bnd / 2.0).exp();
    }
    // Background-weighted contrast.
    let mut wctr = vec![0.0f32; n];
    for p in 0..n {
        let mut s = 0.0;
        for q in 0..n {
            let (dx, dy) = (pos[p][0] - pos[q][0], pos[p][1] - pos[q][1]);
            let wspa = (-(dx * dx + dy * dy) / (2.0 * SIGMA_SPA * SIGMA_SPA)).exp();
            s += dapp(p, q) * wspa * wbg[q];
        }
        wctr[p] = s;
    }
    let mx = wctr.iter().copied().fold(0.0f32, f32::max).max(1e-6);
    let wfg: Vec<f32> = wctr.iter().map(|v| v / mx).collect();
    // Saliency optimisation: min Σ wbg s² + Σ wfg (s−1)² + Σ_{i~j} w_ij (s_i − s_j)² (Gauss–Seidel).
    let wij: Vec<Vec<f32>> = (0..n).map(|p| adj[p].iter().map(|&q| (-dapp(p, q).powi(2) / (2.0 * SIGMA_CLR * SIGMA_CLR)).exp() + 0.1).collect()).collect();
    let mut s = wfg.clone();
    for _ in 0..200 {
        for p in 0..n {
            let (mut num, mut den) = (wfg[p], wbg[p] + wfg[p]);
            for (q, wq) in adj[p].iter().zip(&wij[p]) {
                num += wq * s[*q];
                den += wq;
            }
            s[p] = num / den.max(1e-6);
        }
    }
    // Frequency-tuned saliency.
    let mut mean_all = [0.0f64; 3];
    for c in &lab {
        for k in 0..3 {
            mean_all[k] += c[k] as f64;
        }
    }
    let mean_all = mean_all.map(|v| (v / lab.len() as f64) as f32);
    let blurred = super::RgbImage { w, h, px: lab.clone() };
    let blurred = blur_lab(&blurred);
    let ft: Vec<f32> = blurred.px.iter().map(|c| super::d2(*c, mean_all).sqrt()).collect();
    let ftmax = ft.iter().copied().fold(0.0f32, f32::max).max(1e-6);
    let smin = s.iter().copied().fold(f32::MAX, f32::min);
    let smax = s.iter().copied().fold(f32::MIN, f32::max);
    let srange = (smax - smin).max(1e-6);
    let mut out: Vec<f32> = sp.labels.iter().zip(&ft).map(|(l, f)| 0.75 * (s[*l as usize] - smin) / srange + 0.25 * f / ftmax).collect();
    let (lo, hi) = out.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let r = (hi - lo).max(1e-6);
    for v in &mut out {
        *v = (*v - lo) / r;
    }
    out
}

fn blur_lab(img: &super::RgbImage) -> super::RgbImage {
    let (w, h) = (img.w, img.h);
    let mut cur = img.clone();
    for _ in 0..2 {
        let mut tmp = cur.clone();
        for y in 0..h {
            for x in 0..w {
                let (l, c, r) = (cur.at(x.saturating_sub(1), y), cur.at(x, y), cur.at((x + 1).min(w - 1), y));
                tmp.px[y * w + x] = [0, 1, 2].map(|k| (l[k] + 2.0 * c[k] + r[k]) / 4.0);
            }
        }
        for y in 0..h {
            for x in 0..w {
                let (u, c, d) = (tmp.at(x, y.saturating_sub(1)), tmp.at(x, y), tmp.at(x, (y + 1).min(h - 1)));
                cur.px[y * w + x] = [0, 1, 2].map(|k| (u[k] + 2.0 * c[k] + d[k]) / 4.0);
            }
        }
    }
    cur
}

#[derive(PartialEq)]
struct Item(f32, usize);
impl Eq for Item {}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0)
    }
}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn dijkstra(src: usize, adj: &[Vec<usize>], wt: &dyn Fn(usize, usize) -> f32, dist: &mut [f32]) {
    dist.fill(f32::INFINITY);
    dist[src] = 0.0;
    let mut heap = BinaryHeap::new();
    heap.push(Item(0.0, src));
    while let Some(Item(d, u)) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        for &v in &adj[u] {
            let nd = d + wt(u, v);
            if nd < dist[v] {
                dist[v] = nd;
                heap.push(Item(nd, v));
            }
        }
    }
}

/// Otsu's threshold of values in `0..=1` (256 bins).
pub fn otsu(v: &[f32]) -> f32 {
    let mut hist = [0u64; 256];
    for x in v {
        hist[(x.clamp(0.0, 1.0) * 255.0).round() as usize] += 1;
    }
    let total = v.len() as f64;
    let sum: f64 = hist.iter().enumerate().map(|(i, c)| i as f64 * *c as f64).sum();
    let (mut wb, mut sb, mut best, mut bt) = (0.0f64, 0.0f64, -1.0f64, 0usize);
    for (t, c) in hist.iter().enumerate() {
        wb += *c as f64;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sb += t as f64 * *c as f64;
        let (mb, mf) = (sb / wb, (sum - sb) / wf);
        let between = wb * wf * (mb - mf) * (mb - mf);
        if between > best {
            best = between;
            bt = t;
        }
    }
    (bt as f32 + 0.5) / 255.0
}
