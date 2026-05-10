//! メニュー用の基図形プリセット種別と、各プリセットに対応する線分リストを決定的に生成する処理を定義する。
//!
//! メニューの「Segment」「Triangle」「Star」などから、読みやすい出発線分をすぐそろえる。自分で線を描く選択肢は別にありつつ、
//! 定番の線分セットを名前で載せられることでフラクタル試行をすばやく始められる。また同じプリセットならいつも同じ線分並びになるので、再現や比較の土台にもなる。
//!
//! 開発側では、このカタログと作図手順だけをひとつの場所へ閉じる。[`super::shape::Line`] / [`super::shape::BaseShape`] は状態に依存しない表現のみを担うのに対し、ここでは「名前に対応する [`super::shape::Line`] の列」を決定的に構築する。
//! ワークスペースへの適用や Undo は呼び出し側の責務とし、出力は線分列のみとする。頂点は [-1, 1] に収まる規約で定義している。

use glam::Vec2;

use super::shape::Line;

/// 外心が原点の図形（線分・正三角形・正多角形・星）で共有する頂点の外接円半径。
/// 単位正方の内接円と一致するので、頂点座標は常に [-1, 1] に収まる。
const CIRCUMRADIUS: f32 = 1.0;

/// 軸そろえ正方形の中心から辺までの距離（半辺）。`±1` の辺に接する。
const SQUARE_HALF_SIDE: f32 = 1.0;

/// UI メニューに並べる、基図形の定型パターン識別子。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseShapePreset {
    /// 水平な直径 2 の 1 本の線分（点対称の出発図形）。
    Segment,
    /// 下辺が水平な正三角形の 3 辺。
    Triangle,
    /// 軸にそった正方形の 4 辺。
    Square,
    /// 正五角形の 5 辺（頂点を反転して向きを揃える）。
    Pentagon,
    /// 正六角形の 6 辺。
    Hexagon,
    /// 正五角形の頂点を飛び飛びに結んだ五角星（ペンタグラム）。
    Star,
    /// 原点を角とし、+x と +y に沿う 2 本の線分からなる L 字。
    LShape,
}

impl BaseShapePreset {
    /// メニューに載せるすべてのプリセットを、アプリ側がループしやすい順で並べたスライス。
    pub const ALL: &'static [BaseShapePreset] = &[
        BaseShapePreset::Segment,
        BaseShapePreset::Triangle,
        BaseShapePreset::Square,
        BaseShapePreset::Pentagon,
        BaseShapePreset::Hexagon,
        BaseShapePreset::Star,
        BaseShapePreset::LShape,
    ];

    /// UI 表示用の短い英語ラベル。
    ///
    /// # Returns
    ///
    /// リテラルに近い静的文字列（翻訳レイヤは呼び出し側）。
    pub fn label(self) -> &'static str {
        match self {
            BaseShapePreset::Segment => "Segment",
            BaseShapePreset::Triangle => "Triangle",
            BaseShapePreset::Square => "Square",
            BaseShapePreset::Pentagon => "Pentagon",
            BaseShapePreset::Hexagon => "Hexagon",
            BaseShapePreset::Star => "Star (pentagram)",
            BaseShapePreset::LShape => "L-shape",
        }
    }

    /// このプリセットに対応する基図形の線分リストを新規に構築する。
    ///
    /// 返したベクトルを [`BaseShape`](crate::core::shape::BaseShape) の `lines` フィールドへ代入するなどしてワークスペースへ反映する。
    ///
    /// # Returns
    ///
    /// 正規化座標上の [`Line`] の集合。空になることは通常ない（内部不整合時のみ `Star` が空になりうる）。
    pub fn lines(self) -> Vec<Line> {
        match self {
            BaseShapePreset::Segment => vec![Line {
                a: Vec2::new(-CIRCUMRADIUS, 0.0),
                b: Vec2::new(CIRCUMRADIUS, 0.0),
            }],
            BaseShapePreset::Triangle => {
                // 正三角形・下辺水平・外心が原点
                let r = CIRCUMRADIUS;
                let v0 = Vec2::new(0.0, r);
                let a = 7.0 * std::f32::consts::FRAC_PI_6; // 210°
                let v1 = Vec2::new(r * a.cos(), r * a.sin());
                let b = 11.0 * std::f32::consts::FRAC_PI_6; // 330°
                let v2 = Vec2::new(r * b.cos(), r * b.sin());
                vec![
                    Line { a: v0, b: v1 },
                    Line { a: v1, b: v2 },
                    Line { a: v2, b: v0 },
                ]
            }
            BaseShapePreset::Square => {
                let s = SQUARE_HALF_SIDE;
                let bl = Vec2::new(-s, -s);
                let br = Vec2::new(s, -s);
                let tr = Vec2::new(s, s);
                let tl = Vec2::new(-s, s);
                vec![
                    Line { a: bl, b: br },
                    Line { a: br, b: tr },
                    Line { a: tr, b: tl },
                    Line { a: tl, b: bl },
                ]
            }
            BaseShapePreset::Pentagon => {
                let v: Vec<Vec2> = regular_polygon_vertices(5)
                    .into_iter()
                    .map(|p| -p)
                    .collect();
                cycle_edges(&v)
            }
            BaseShapePreset::Hexagon => {
                let v = regular_polygon_vertices(6);
                cycle_edges(&v)
            }
            BaseShapePreset::Star => {
                let v: Vec<Vec2> = regular_polygon_vertices(5)
                    .into_iter()
                    .map(|p| -p)
                    .collect();
                if v.len() != 5 {
                    return vec![];
                }
                let order = [0usize, 2, 4, 1, 3, 0];
                order
                    .windows(2)
                    .map(|w| Line {
                        a: v[w[0]],
                        b: v[w[1]],
                    })
                    .collect()
            }
            BaseShapePreset::LShape => {
                // 角を原点に、+x へ水平・+y へ垂直の L。
                let a = CIRCUMRADIUS;
                vec![
                    Line {
                        a: Vec2::ZERO,
                        b: Vec2::new(a, 0.0),
                    },
                    Line {
                        a: Vec2::ZERO,
                        b: Vec2::new(0.0, a),
                    },
                ]
            }
        }
    }
}

/// 正 n 角形の頂点を、下辺が水平になる向きで並べる（最初の頂点が最下点）。
fn regular_polygon_vertices(n: usize) -> Vec<Vec2> {
    let n = n as f32;
    (0..n as usize)
        .map(|k| {
            let a = -std::f32::consts::FRAC_PI_2 + (k as f32) * std::f32::consts::TAU / n;
            Vec2::new(CIRCUMRADIUS * a.cos(), CIRCUMRADIUS * a.sin())
        })
        .collect()
}

fn cycle_edges(vertices: &[Vec2]) -> Vec<Line> {
    let n = vertices.len();
    if n < 2 {
        return vec![];
    }
    (0..n)
        .map(|i| Line {
            a: vertices[i],
            b: vertices[(i + 1) % n],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_is_unit_horizontal_diameter() {
        let lines = BaseShapePreset::Segment.lines();
        assert_eq!(lines.len(), 1);
        assert!((lines[0].a - Vec2::new(-1.0, 0.0)).length() < 1e-5);
        assert!((lines[0].b - Vec2::new(1.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn square_is_closed_quad() {
        let lines = BaseShapePreset::Square.lines();
        assert_eq!(lines.len(), 4);
        assert!((lines[3].b - lines[0].a).length() < 1e-5);
    }
}
