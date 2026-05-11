//! プリセット一覧用 PNG を `assets/fractalium/preset_thumbnails/` に出力する開発用ツール。
//!
//! [`FractalPreset::build`](fractalium::fractal_presets::FractalPreset::build) の状態を
//! [`for_each_fractal_line_segment`](fractalium::core::fractal_line_walk::for_each_fractal_line_segment)
//! で展開し、Result パネルと同じ色相（`Hsla::new(hue, 0.88, 0.58, 1.0)`）でラスタ化する。
//!
//! リポジトリルートで `cargo run --example gen_preset_thumbnails` を実行する。
//! Cargo の **example** ターゲットに置く。`[[bin]]` を増やすと `trunk build` が「複数のアーティファクト」で失敗するため。
//!
//! # ファイル名
//!
//! [`fractalium::ui::preset_picker::thumbnails`] の `include_bytes!` と同じ *_snake_*.png。
//! 解像度 352×264（論理サイズ約 158×122 の約 2 倍）は `egui` の縮小表示でシャープになるよう予め粗さを確保した固定値。

use bevy_color::{ColorToPacked as _, Hsla, LinearRgba, Srgba};
use glam::Vec2;
use image::{Rgba, RgbaImage};

use fractalium::app::session::FractalState;
use fractalium::core::fractal_line_walk::{FractalLineSegment, for_each_fractal_line_segment};
use fractalium::fractal_presets::FractalPreset;

/// カード非選択時の背景に揃えた地色（`tiles` の `from_rgb(36, 40, 48)`）。
const BG: Rgba<u8> = Rgba([36, 40, 48, 255]);
/// 画像端から内容までの余白（ピクセル・基準画像の約 2 倍スケール）。
const PIXEL_MARGIN: f32 = 16.0;
/// 線の見かけ太さ（ピクセル、直径相当。解像度 2 倍に合わせて同倍率）。
const STROKE_PX: f32 = 4.5;
/// 固定出力幅（[`thumbnails`](fractalium::ui::preset_picker::thumbnails) の `include_bytes!` と一致）。
const OUT_W: u32 = 352;
/// 固定出力高さ。
const OUT_H: u32 = 264;

fn main() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/fractalium/preset_thumbnails"
    );
    std::fs::create_dir_all(dir).expect("mkdir fractal preset thumbnails");

    for &preset in FractalPreset::ALL {
        let path = format!("{dir}/{}.png", png_stem(preset));
        let img = render_state(&preset.build());
        img.save(path.as_str())
            .unwrap_or_else(|e| panic!("save {path}: {e}"));
    }

    let new_path = format!("{dir}/new.png");
    let new_img = render_new_tile();
    new_img
        .save(new_path.as_str())
        .unwrap_or_else(|e| panic!("save {new_path}: {e}"));
}

/// `FractalPreset` に対応するファイル名（拡張子なし）。`thumbnails.rs` の PNG 名と一致させる。
fn png_stem(p: FractalPreset) -> &'static str {
    match p {
        FractalPreset::SierpinskiTriangle => "sierpinski_triangle",
        FractalPreset::KochCurve => "koch_curve",
        FractalPreset::Vicsek => "vicsek",
        FractalPreset::HeighwayDragon => "heighway_dragon",
        FractalPreset::LevyCCurve => "levy_c",
        FractalPreset::PythagorasTree => "pythagoras_tree",
        FractalPreset::SierpinskiHexagon => "sierpinski_hexagon",
        FractalPreset::SierpinskiStar => "sierpinski_star",
        FractalPreset::Terdragon => "terdragon",
        FractalPreset::HalCycloneTriangle => "hal_cyclone_triangle",
        FractalPreset::HalWing => "hal_wing",
        FractalPreset::HalTree => "hal_tree",
        FractalPreset::HalVStar => "hal_v_star",
        FractalPreset::HalMosaicWindow => "hal_mosaic_window",
    }
}

/// Result パネルと同じ規則で色相から sRGB 8bit を得る。
fn line_rgb_u8(hue_degrees: f32) -> [u8; 3] {
    let lin = LinearRgba::from(Hsla::new(hue_degrees.rem_euclid(360.0), 0.88, 0.58, 1.0));
    Srgba::from(lin).to_u8_array_no_alpha()
}

fn collect_segments(state: &FractalState) -> Vec<FractalLineSegment> {
    let mut out = Vec::new();
    for_each_fractal_line_segment(
        state.depth,
        &state.base_shape.lines,
        &state.replicas,
        state.show_all_generations,
        |s| out.push(s),
    );
    out
}

fn bounds_endpoints(segments: &[FractalLineSegment]) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    let mut any = false;
    for s in segments {
        for p in [s.a, s.b] {
            min = min.min(p);
            max = max.max(p);
            any = true;
        }
    }
    if !any {
        return None;
    }
    let mut extent = max - min;
    if extent.x < 1e-4 {
        extent.x = 0.25;
    }
    if extent.y < 1e-4 {
        extent.y = 0.25;
    }
    let center = (min + max) * 0.5;
    let half = extent * 0.5;
    Some((center - half, center + half))
}

/// フラクタル正規化座標からサムネ画像ピクセル（左上原点・y 下向き）への等方フィット。
struct WorldToPx {
    /// バウンディングのワールド最小座標。
    min: Vec2,
    /// バウンディングのワールド最大座標。
    max: Vec2,
    /// ワールド 1 に対するピクセル換算係数。
    scale: f32,
    /// フィット済み描画域の左上 x（ピクセル）。
    ox: f32,
    /// フィット済み描画域の左上 y（ピクセル）。
    oy: f32,
}

impl WorldToPx {
    fn new(min_c: Vec2, max_c: Vec2, tw: u32, th: u32, pad: f32) -> Self {
        let inner_w = tw as f32 - 2.0 * pad;
        let inner_h = th as f32 - 2.0 * pad;
        let extent = max_c - min_c;
        let sx = inner_w / extent.x.max(1e-6);
        let sy = inner_h / extent.y.max(1e-6);
        let scale = sx.min(sy);
        let used_w = extent.x * scale;
        let used_h = extent.y * scale;
        let ox = pad + (inner_w - used_w) * 0.5;
        let oy = pad + (inner_h - used_h) * 0.5;
        Self {
            min: min_c,
            max: max_c,
            scale,
            ox,
            oy,
        }
    }

    fn map(&self, w: Vec2) -> Vec2 {
        let x = self.ox + (w.x - self.min.x) * self.scale;
        let y = self.oy + (self.max.y - w.y) * self.scale;
        Vec2::new(x, y)
    }
}

fn cross2(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn point_in_convex_quad(p: Vec2, q: &[Vec2; 4]) -> bool {
    let mut sign: i8 = 0;
    for i in 0..4 {
        let a = q[i];
        let b = q[(i + 1) % 4];
        let c = cross2(b - a, p - a);
        if c.abs() < 1e-8 {
            continue;
        }
        let s = c.signum() as i8;
        if sign == 0 {
            sign = s;
        } else if sign != s {
            return false;
        }
    }
    true
}

fn stroke_quad_px(a: Vec2, b: Vec2, half_w: f32) -> Option<[Vec2; 4]> {
    let d = b - a;
    if d.length_squared() < 1e-12 {
        return None;
    }
    let n = Vec2::new(-d.y, d.x).normalize() * half_w;
    Some([a - n, a + n, b + n, b - n])
}

fn fill_quad_pixels(img: &mut RgbaImage, corners: &[Vec2; 4], rgb: [u8; 3]) {
    let mut minx = i32::MAX;
    let mut miny = i32::MAX;
    let mut maxx = i32::MIN;
    let mut maxy = i32::MIN;
    for p in corners {
        minx = minx.min(p.x.floor() as i32);
        miny = miny.min(p.y.floor() as i32);
        maxx = maxx.max(p.x.ceil() as i32);
        maxy = maxy.max(p.y.ceil() as i32);
    }
    let w = img.width() as i32;
    let h = img.height() as i32;
    minx = minx.max(0);
    miny = miny.max(0);
    maxx = maxx.min(w - 1);
    maxy = maxy.min(h - 1);
    for y in miny..=maxy {
        for x in minx..=maxx {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            if point_in_convex_quad(p, corners) {
                img.put_pixel(x as u32, y as u32, Rgba([rgb[0], rgb[1], rgb[2], 255]));
            }
        }
    }
}

fn render_state(state: &FractalState) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(OUT_W, OUT_H, BG);
    let segments = collect_segments(state);
    let Some((min_c, max_c)) = bounds_endpoints(&segments) else {
        return img;
    };
    let mapper = WorldToPx::new(min_c, max_c, OUT_W, OUT_H, PIXEL_MARGIN);
    let half_px = STROKE_PX * 0.5;

    for seg in &segments {
        let pa = mapper.map(seg.a);
        let pb = mapper.map(seg.b);
        let Some(quad) = stroke_quad_px(pa, pb, half_px) else {
            continue;
        };
        let rgb = line_rgb_u8(seg.hue_degrees);
        fill_quad_pixels(&mut img, &quad, rgb);
    }
    img
}

fn render_new_tile() -> RgbaImage {
    let mut img = RgbaImage::from_pixel(OUT_W, OUT_H, BG);
    let cx = OUT_W as f32 * 0.5;
    let cy = OUT_H as f32 * 0.5;
    let arm = (OUT_W.min(OUT_H) as f32) * 0.2;
    let half = STROKE_PX * 0.5;
    let rgb = [108, 112, 128];
    if let Some(qh) = stroke_quad_px(Vec2::new(cx - arm, cy), Vec2::new(cx + arm, cy), half) {
        fill_quad_pixels(&mut img, &qh, rgb);
    }
    if let Some(qv) = stroke_quad_px(Vec2::new(cx, cy - arm), Vec2::new(cx, cy + arm), half) {
        fill_quad_pixels(&mut img, &qv, rgb);
    }
    img
}
