//! Seed（基図形）用の正規化座標プリセット。
//! 座標は `FractalState` と同じ [-1, 1]² を前提とし、はみ出さない範囲で最大サイズにする。

use bevy::prelude::Vec2;

use crate::state::Line;

/// 外心が原点の図形（線分・正三角形・正多角形・星）で共有する頂点の外接円半径。
/// 単位正方の内接円と一致するので、頂点座標は常に [-1, 1] に収まる。
const CIRCUMRADIUS: f32 = 1.0;

/// 軸そろえ正方形の中心から辺までの距離（半辺）。`±1` の辺に接する。
const SQUARE_HALF_SIDE: f32 = 1.0;

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

/// メニューに載せるプリセット（最優先・高優先を実装）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaseShapePreset {
    Segment,
    Triangle,
    Square,
    Pentagon,
    Hexagon,
    Star,
    LShape,
}

impl BaseShapePreset {
    pub const ALL: &'static [BaseShapePreset] = &[
        BaseShapePreset::Segment,
        BaseShapePreset::Triangle,
        BaseShapePreset::Square,
        BaseShapePreset::Pentagon,
        BaseShapePreset::Hexagon,
        BaseShapePreset::Star,
        BaseShapePreset::LShape,
    ];

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
