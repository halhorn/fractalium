//! プリセット一覧用 PNG を `assets/fractalium/preset_thumbnails/` に出力する開発用ツール。
//!
//! リポジトリルートで `cargo run --example gen_preset_thumbnails` を実行する。
//! Cargo の **example** ターゲットに置く。`[[bin]]` を増やすと `trunk build` が「複数のアーティファクト」で失敗するため。

use image::Rgba;
use image::RgbaImage;

fn main() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fractalium/preset_thumbnails");
    std::fs::create_dir_all(dir).expect("mkdir fractal preset thumbnails");

    let entries: &[(&str, [u8; 3])] = &[
        ("sierpinski_triangle", [210, 90, 92]),
        ("koch_curve", [98, 160, 210]),
        ("vicsek", [120, 200, 130]),
        ("heighway_dragon", [200, 150, 80]),
        ("levy_c", [170, 120, 200]),
        ("sierpinski_carpet", [210, 200, 90]),
        ("pythagoras_tree", [90, 180, 140]),
        ("sierpinski_hexagon", [230, 120, 150]),
        ("sierpinski_star", [160, 160, 220]),
        ("binary_fractal_tree", [140, 150, 100]),
        ("terdragon", [100, 130, 200]),
        ("new", [55, 58, 64]),
    ];

    let tw = 176_u32;
    let th = 132_u32;

    for &(name, rgb) in entries {
        let shaded = name != "new";
        let img = RgbaImage::from_fn(tw, th, |x, y| {
            let gx = x as f32 / tw.saturating_sub(1).max(1) as f32;
            let gy = y as f32 / th.saturating_sub(1).max(1) as f32;
            let mut r = rgb[0] as f32;
            let mut g = rgb[1] as f32;
            let mut b = rgb[2] as f32;
            if shaded {
                r *= 0.55 + 0.45 * gx;
                g *= 0.55 + 0.45 * gy;
                b *= 0.62 + 0.38 * (1.0 - gx);
            }
            Rgba([
                r.clamp(0.0, 255.0) as u8,
                g.clamp(0.0, 255.0) as u8,
                b.clamp(0.0, 255.0) as u8,
                255,
            ])
        });
        let path = format!("{dir}/{name}.png");
        img.save(path.as_str()).unwrap_or_else(|e| panic!("save {path}: {e}"));
    }
}
