//! フラクタル本体（Result キャンバス側）の再帰計算と描画を提供するモジュール。
//!
//! 描画手順は次の通り：
//! 1. 恒等変換から始めて、深さ `depth - 1` 回まで複製変換を再帰的に合成
//! 2. リーフで基図形の各線分にその合成変換を適用
//! 3. 各リーフに割り当てられた色相で線を描画

use bevy::color::Hsla;
use bevy::prelude::*;

use crate::result_layer;
use crate::state::{FractalState, Line, Replica};

/// Result キャンバス専用のギズモグループ。
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ResultGizmos;

/// Result キャンバスへフラクタルを描画する一連のシステムを登録するプラグイン。
pub struct FractalPlugin;

impl Plugin for FractalPlugin {
    fn build(&self, app: &mut App) {
        let mut config = bevy::gizmos::config::GizmoConfig {
            render_layers: result_layer(),
            ..Default::default()
        };
        config.line.width = 2.0;

        app.insert_gizmo_config(ResultGizmos, config)
            .add_systems(Update, draw_fractal_result);
    }
}

/// HSL 色相からリニア RGB 色を生成する。彩度・輝度は固定。
fn hue_to_color(hue: f32) -> Color {
    let lin = LinearRgba::from(Hsla::new(hue % 360.0, 0.88, 0.58, 1.0));
    Color::from(lin)
}

/// Edit パネルで使う、複製インデックス由来の色。
/// 黄金比に近い角度（137.508°）で色相をずらすことで、隣り合う複製の色を区別しやすくしている。
pub fn replica_color(i: usize) -> LinearRgba {
    LinearRgba::from(Hsla::new((i as f32 * 137.508) % 360.0, 0.85, 0.60, 1.0))
}

/// Result キャンバスにフラクタルの全リーフ線分を描画する。
fn draw_fractal_result(state: Res<FractalState>, mut gizmos: Gizmos<ResultGizmos>) {
    let mut segments: Vec<(Vec2, Vec2, f32)> = Vec::new();
    // 最上位の複製で 360° の色相を分け合うように、初期 hue=0、hue_step=360 で開始する。
    collect_fractal_segments(
        state.depth,
        Replica::identity(),
        &state.base_shape.lines,
        &state.replicas,
        &mut segments,
        0.0,
        360.0,
    );
    for &(a, b, hue) in &segments {
        gizmos.line_2d(a, b, hue_to_color(hue));
    }
}

/// フラクタルを再帰的に展開し、リーフ線分（変換後の座標）と色相を `out` に積む。
///
/// - `depth`: 残りの再帰回数。1 になったらリーフとして基図形を吐き出す。
/// - `transform`: ここまでに合成された変換。
/// - `hue` / `hue_step`: この部分木に割り当てられた色相のベース値と幅。
///   各レベルで子 replica が等分し、最上位の replica が 360° を分け合う。
fn collect_fractal_segments(
    depth: u32,
    transform: Replica,
    lines: &[Line],
    replicas: &[Replica],
    out: &mut Vec<(Vec2, Vec2, f32)>,
    hue: f32,
    hue_step: f32,
) {
    if depth <= 1 {
        for line in lines {
            out.push((transform.apply(line.a), transform.apply(line.b), hue));
        }
        return;
    }
    let n = replicas.len() as f32;
    let child_step = hue_step / n;
    for (i, replica) in replicas.iter().enumerate() {
        collect_fractal_segments(
            depth - 1,
            transform.compose(*replica),
            lines,
            replicas,
            out,
            hue + i as f32 * child_step,
            child_step,
        );
    }
}
