//! Measurement aid, not shipped: runs Select Subject with several tunings over a folder of raw
//! RGB images and writes each selection as raw bytes, for scoring against reference masks.
//!
//! `cargo run -p photocraft-algo --example ss_probe -- <in-dir> <out-dir> <variants.txt>`
//!
//! `<in-dir>` holds `<name>.rgb` (8-bit RGB, row-major) beside `<name>.dim` ("width height").
//! Each line of the variants file is `name bg_sides portrait skin_gate shoulder hair [band]`,
//! where `portrait` is `off`, `probable` or `core`, followed by any of the words `largest` (the
//! band hangs under the largest part of the first selection only) and `confine` (the second cut
//! may only add inside the band). Output: `<out-dir>/<variant>/<name>.mask`
//! (width x height bytes, 255 = selected) or `<name>.none`, and `times.txt`.
use std::time::Instant;

use photocraft_algo::segment::subject::{self, Portrait, SubjectTuning};
use photocraft_algo::segment::{ImageSampler, RgbImage};
use photocraft_geom::Rect;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (indir, outdir, variants) = (&args[1], &args[2], &args[3]);
    let mut names: Vec<String> = std::fs::read_dir(indir)
        .expect("in-dir")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".rgb")).map(str::to_owned))
        .collect();
    names.sort();
    let images: Vec<(String, RgbImage)> = names
        .into_iter()
        .map(|n| {
            let dim = std::fs::read_to_string(format!("{indir}/{n}.dim")).expect("dim");
            let mut it = dim.split_whitespace().map(|v| v.parse::<usize>().expect("number"));
            let (w, h) = (it.next().unwrap(), it.next().unwrap());
            let raw = std::fs::read(format!("{indir}/{n}.rgb")).expect("rgb");
            assert_eq!(raw.len(), w * h * 3, "{n}");
            let px = raw.chunks_exact(3).map(|c| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]).collect();
            (n, RgbImage { w, h, px })
        })
        .collect();
    for line in std::fs::read_to_string(variants).expect("variants").lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 6 || f[0].starts_with('#') {
            continue;
        }
        let tune = SubjectTuning {
            bg_sides: f[1].parse().expect("bg_sides"),
            portrait: match f[2] {
                "off" => Portrait::Off,
                "probable" => Portrait::Probable,
                "core" => Portrait::Core,
                other => panic!("portrait {other}"),
            },
            skin_gate: f[3].parse().expect("skin_gate"),
            shoulder: f[4].parse().expect("shoulder"),
            hair: f[5].parse().expect("hair"),
            band_under_largest: f[6..].contains(&"largest"),
            confine: f[6..].contains(&"confine"),
        };
        let dir = format!("{outdir}/{}", f[0]);
        std::fs::create_dir_all(&dir).expect("out-dir");
        let mut times = String::new();
        for (n, img) in &images {
            let (w, h) = (img.w, img.h);
            let s = ImageSampler { img, origin: (0, 0) };
            let t0 = Instant::now();
            let reg = subject::select_subject_tuned(&s, Rect::new(0, 0, w as i32, h as i32), &tune);
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            times.push_str(&format!("{n} {ms:.0}\n"));
            match reg {
                None => std::fs::write(format!("{dir}/{n}.none"), b"").expect("write"),
                Some(r) => {
                    let mut full = vec![0u8; w * h];
                    let (bw, bh) = (r.bbox.width() as usize, r.bbox.height() as usize);
                    for y in 0..bh {
                        let (dy, dx) = ((r.bbox.y0 as usize) + y, r.bbox.x0 as usize);
                        full[dy * w + dx..dy * w + dx + bw].copy_from_slice(&r.mask[y * bw..(y + 1) * bw]);
                    }
                    std::fs::write(format!("{dir}/{n}.mask"), full).expect("write");
                }
            }
        }
        std::fs::write(format!("{dir}/times.txt"), times).expect("write");
        println!("{} done", f[0]);
    }
}
