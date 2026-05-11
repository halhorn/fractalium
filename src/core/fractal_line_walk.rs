//! Result パネルとオフラインのサムネ生成で共有する、フラクタル線分の再帰走査。
//!
//! Bevy・egui に依存せず、ワールド座標の線分と色相（度）だけを通知する。色の解釈は
//! [`crate::ui::canvas::result::scene`] の `hue_to_linear_rgba` と揃える。

use glam::Vec2;

use crate::core::shape::{Line, Replica};

/// 末端（または `show_all_generations` 時の途中世代）で描画対象になる 1 線分。
#[derive(Clone, Copy, Debug)]
pub struct FractalLineSegment {
    /// 始点（正規化キャンバス座標）。
    pub a: Vec2,
    /// 終点。
    pub b: Vec2,
    /// Result パネルと同じ色相レンジ（度）。`Hsla::new(hue, 0.88, 0.58, 1.0)` の入力。
    pub hue_degrees: f32,
}

/// `depth`・基図形・レプリカから、描画すべき線分を再帰的に列挙する。
///
/// # 引数
/// - `depth` — フラクタル状態の深さ（1 なら基図形のみ）。
/// - `lines` — 基図形の線分。
/// - `replicas` — IFS の各枝。
/// - `show_all_generations` — 真なら末端以外の世代の図形も含める。
/// - `visit` — 各線分ごとのコールバック。
///
/// # 戻り値
/// なし。列挙は `visit` の副作用。
pub fn for_each_fractal_line_segment(
    depth: u32,
    lines: &[Line],
    replicas: &[Replica],
    show_all_generations: bool,
    mut visit: impl FnMut(FractalLineSegment),
) {
    walk(
        depth,
        Replica::identity(),
        lines,
        replicas,
        show_all_generations,
        &mut visit,
        0.0,
        360.0,
    );
}

/// [`for_each_fractal_line_segment`] の内部再帰。`visit` に渡る座標は `transform` 適用後。
///
/// # 引数
/// - `depth` — 残り再帰深度。
/// - `transform` — 現在ノードまでの合成変換。
/// - `lines` / `replicas` — フラクタル定義。
/// - `show_all_generations` — 途中世代も列挙するか。
/// - `visit` — 線分ごとのコールバック。
/// - `hue` / `hue_step` — 現在ノードの色相（度）と子へ割り当てる幅。
///
/// # 戻り値
/// なし。
fn walk(
    depth: u32,
    transform: Replica,
    lines: &[Line],
    replicas: &[Replica],
    show_all_generations: bool,
    visit: &mut impl FnMut(FractalLineSegment),
    hue: f32,
    hue_step: f32,
) {
    let is_leaf = depth <= 1 || replicas.is_empty();

    if is_leaf || show_all_generations {
        for line in lines {
            let a = transform.apply(line.a);
            let b = transform.apply(line.b);
            visit(FractalLineSegment {
                a,
                b,
                hue_degrees: hue,
            });
        }
    }

    if is_leaf {
        return;
    }

    let n = replicas.len() as f32;
    let child_step = hue_step / n;
    for (i, replica) in replicas.iter().enumerate() {
        walk(
            depth - 1,
            transform.compose(*replica),
            lines,
            replicas,
            show_all_generations,
            visit,
            hue + i as f32 * child_step,
            child_step,
        );
    }
}
