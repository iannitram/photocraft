//! The step budget of the automatic max-flow choice ([`grid_cut_within`]): a cut that runs out
//! of steps is redone by Boykov–Kolmogorov, an ordinary cut is left alone, a cancel still cancels.
//!
//! Every picture here is made up by code from a seed. Nothing is taken from any real image.
use super::*;

/// A made-up edge band, the shape `refine_band` cuts: a `w` × `h` picture of 8-bit-like colours
/// (a soft gradient plus noise), the left `edge` columns fixed subject, the right `edge` columns
/// fixed background, every pixel between free, with costs spread like colour-model costs.
fn band(seed: u64, w: usize, h: usize, edge: usize) -> (RgbImage, Vec<f32>, Vec<f32>, Vec<u8>, f32) {
    let mut rng = Rng::new(seed);
    let mut unit = || (rng.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
    let mut px = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let base = [x as f32 / w as f32, y as f32 / h as f32, 0.5];
            px.push([0, 1, 2].map(|k| ((base[k] + 0.6 * (unit() - 0.5)).clamp(0.0, 1.0) * 255.0).round() / 255.0));
        }
    }
    let img = RgbImage { w, h, px };
    let n = w * h;
    let cost_fg: Vec<f32> = (0..n).map(|_| -10.0 + 41.0 * unit()).collect();
    let cost_bg: Vec<f32> = (0..n).map(|_| -10.0 + 41.0 * unit()).collect();
    let fixed: Vec<u8> = (0..n).map(|i| if i % w < edge { HARD_FG } else if i % w >= w - edge { HARD_BG } else { FREE }).collect();
    let beta = contrast_beta(&img);
    (img, cost_fg, cost_bg, fixed, beta)
}

/// 72 × 72 with 6 fixed columns each side: 4320 free pixels, enough for the automatic choice to
/// take push/relabel (4096 nodes or more, terminals both ways).
const BIG: (usize, usize, usize) = (72, 72, 6);

#[test]
fn a_cut_out_of_steps_is_redone_by_boykov_kolmogorov_and_says_so() {
    for seed in [1u64, 2, 3] {
        let (img, cost_fg, cost_bg, fixed, beta) = band(seed, BIG.0, BIG.1, BIG.2);
        let weight = |d2: f32| 50.0 * (-beta * d2).exp();
        let bk = grid_cut_impl::<true>(&img, &cost_fg, &cost_bg, &fixed, weight, &|| false, Solver::Bk).unwrap();
        // A budget of no asks at all: the automatic solver is stopped at its first ask.
        let started = std::time::Instant::now();
        let (cut, handover) = grid_cut_within::<true>(&img, &cost_fg, &cost_bg, &fixed, weight, &|| false, Some(0));
        assert!(started.elapsed() < std::time::Duration::from_secs(10), "a stopped cut must come back at once");
        assert_eq!(handover, Some(Handover { free: 4320, steps: u64::from(maxflow::CANCEL_EVERY) }), "seed {seed}");
        assert_eq!(cut.as_deref(), Some(bk.as_slice()), "seed {seed}: the redone cut is Boykov–Kolmogorov's");
        // The public entry points take the same road and still answer.
        assert_eq!(grid_cut(&img, &cost_fg, &cost_bg, &fixed, 50.0, beta).len(), img.w * img.h);
    }
}

#[test]
fn the_measured_budget_leaves_ordinary_cuts_alone() {
    for (seed, (w, h, edge)) in [(4u64, BIG), (5, BIG), (6, (40, 30, 3)), (7, (96, 48, 4))] {
        let (img, cost_fg, cost_bg, fixed, beta) = band(seed, w, h, edge);
        let weight = |d2: f32| 50.0 * (-beta * d2).exp();
        let plain = grid_cut_impl::<true>(&img, &cost_fg, &cost_bg, &fixed, weight, &|| false, Solver::Auto).unwrap();
        let (cut, handover) = grid_cut_within::<true>(&img, &cost_fg, &cost_bg, &fixed, weight, &|| false, None);
        assert_eq!(handover, None, "seed {seed}: an ordinary cut finishes far inside its budget");
        assert_eq!(cut, Some(plain), "seed {seed}");
    }
}

#[test]
fn the_budget_is_far_above_what_a_finishing_cut_needs() {
    // Count the asks an ordinary cut makes: the budget must leave it a wide margin.
    for seed in [8u64, 9, 10] {
        let (img, cost_fg, cost_bg, fixed, beta) = band(seed, BIG.0, BIG.1, BIG.2);
        let asks = std::cell::Cell::new(0usize);
        let counting = || {
            asks.set(asks.get() + 1);
            false
        };
        assert!(grid_cut_impl::<true>(&img, &cost_fg, &cost_bg, &fixed, |d2| 50.0 * (-beta * d2).exp(), &counting, Solver::Auto).is_some());
        let free = fixed.iter().filter(|f| **f == FREE).count();
        let budget = free * BUDGET_ASKS_PER_FREE_PIXEL;
        assert!(asks.get() * 10 < budget, "seed {seed}: {} asks against a budget of {budget}", asks.get());
    }
}

#[test]
fn a_cancel_still_cancels_and_is_not_a_hand_over() {
    let (img, cost_fg, cost_bg, fixed, beta) = band(11, BIG.0, BIG.1, BIG.2);
    let weight = |d2: f32| 50.0 * (-beta * d2).exp();
    assert_eq!(grid_cut_within::<true>(&img, &cost_fg, &cost_bg, &fixed, weight, &|| true, None), (None, None));
    assert_eq!(grid_cut_cancellable(&img, &cost_fg, &cost_bg, &fixed, weight, &|| true), None);
    // The previous full scan (Boykov–Kolmogorov throughout) has no budget to run out of.
    let (cut, handover) = grid_cut_within::<false>(&img, &cost_fg, &cost_bg, &fixed, weight, &|| false, Some(0));
    assert!(cut.is_some() && handover.is_none());
}
