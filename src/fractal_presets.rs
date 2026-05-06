//! 基図形・レプリカ・深さをまとめて切り替えるフラクタル全体プリセット。
//! IFS は各レプリカ R に対し `transform.compose(R)` で深さ方向に合成される。

use std::f32::consts::{FRAC_PI_3, FRAC_PI_4, SQRT_2};

use bevy::prelude::Vec2;

use crate::seed_shape::BaseShapePreset;
use crate::state::{FractalState, Line, Replica};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FractalPreset {
    SierpinskiTriangle,
    KochCurve,
    CantorSet,
    Vicsek,
    HeighwayDragon,
    LevyCCurve,
    BinaryTree,
    SierpinskiCarpet,
    PythagorasTree,
    SierpinskiHexagon,
}

impl FractalPreset {
    pub const ALL: &'static [FractalPreset] = &[
        FractalPreset::SierpinskiTriangle,
        FractalPreset::KochCurve,
        FractalPreset::CantorSet,
        FractalPreset::Vicsek,
        FractalPreset::HeighwayDragon,
        FractalPreset::LevyCCurve,
        FractalPreset::BinaryTree,
        FractalPreset::SierpinskiCarpet,
        FractalPreset::PythagorasTree,
        FractalPreset::SierpinskiHexagon,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FractalPreset::SierpinskiTriangle => "Sierpiński triangle",
            FractalPreset::KochCurve => "Koch curve",
            FractalPreset::CantorSet => "Cantor set (line)",
            FractalPreset::Vicsek => "Vicsek (fractal cross)",
            FractalPreset::HeighwayDragon => "Heighway dragon",
            FractalPreset::LevyCCurve => "Lévy C curve",
            FractalPreset::BinaryTree => "Binary tree",
            FractalPreset::SierpinskiCarpet => "Sierpiński carpet",
            FractalPreset::PythagorasTree => "Pythagoras tree",
            FractalPreset::SierpinskiHexagon => "Sierpiński hexagon",
        }
    }

    /// `snap_grid` は呼び出し側で引き継ぐ。ここでは `false` で埋めておく。
    pub fn build(self) -> FractalState {
        match self {
            FractalPreset::SierpinskiTriangle => sierpinski_triangle(),
            FractalPreset::KochCurve => koch_curve(),
            FractalPreset::CantorSet => cantor_set(),
            FractalPreset::Vicsek => vicsek(),
            FractalPreset::HeighwayDragon => heighway_dragon(),
            FractalPreset::LevyCCurve => levy_c(),
            FractalPreset::BinaryTree => binary_tree(),
            FractalPreset::SierpinskiCarpet => sierpinski_carpet(),
            FractalPreset::PythagorasTree => pythagoras_tree(),
            FractalPreset::SierpinskiHexagon => sierpinski_hexagon(),
        }
    }

    /// スクリーンショット・検証用: 環境変数 `FRACTALIUM_BOOT_PRESET` から解決する。
    pub fn from_boot_token(raw: &str) -> Option<Self> {
        let key: String = raw
            .trim()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        match key.as_str() {
            "pythagoras" | "pythagorastree" => Some(Self::PythagorasTree),
            "heighway" | "heighwaydragon" | "dragon" => Some(Self::HeighwayDragon),
            "binary" | "binarytree" => Some(Self::BinaryTree),
            _ => Self::ALL.iter().copied().find(|&p| {
                let label_key: String = p
                    .label()
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .map(|c| c.to_ascii_lowercase())
                    .collect();
                label_key == key
            }),
        }
    }
}

fn state(
    base_shape: Vec<Line>,
    replicas: Vec<Replica>,
    depth: u32,
    show_all_generations: bool,
) -> FractalState {
    FractalState {
        base_shape: crate::state::BaseShape { lines: base_shape },
        replicas,
        depth,
        show_all_generations,
        snap_grid: false,
    }
}

/// `BaseShapePreset::Triangle` と同じ 3 頂点（線分の始点を順に繋ぐ）。
fn triangle_vertices() -> [Vec2; 3] {
    let tri = BaseShapePreset::Triangle.lines();
    [tri[0].a, tri[1].a, tri[2].a]
}

fn hexagon_vertices() -> Vec<Vec2> {
    BaseShapePreset::Hexagon
        .lines()
        .iter()
        .map(|l| l.a)
        .collect()
}

fn sierpinski_triangle() -> FractalState {
    let replicas: Vec<Replica> = triangle_vertices()
        .into_iter()
        .map(|p| Replica {
            translation: p * 0.5,
            rotation: 0.0,
            scale: 0.5,
        })
        .collect();
    state(
        BaseShapePreset::Triangle.lines(),
        replicas,
        6,
        false,
    )
}

/// Koch 曲線: [-1,1] の線分に 4 つの相似（回転込み）。
fn koch_curve() -> FractalState {
    let sqrt3_6 = FRAC_PI_3.sin() / 3.0;
    let replicas = vec![
        Replica {
            translation: Vec2::new(-2.0 / 3.0, 0.0),
            rotation: 0.0,
            scale: 1.0 / 3.0,
        },
        Replica {
            translation: Vec2::new(-1.0 / 6.0, sqrt3_6),
            rotation: FRAC_PI_3,
            scale: 1.0 / 3.0,
        },
        Replica {
            translation: Vec2::new(1.0 / 6.0, sqrt3_6),
            rotation: -FRAC_PI_3,
            scale: 1.0 / 3.0,
        },
        Replica {
            translation: Vec2::new(2.0 / 3.0, 0.0),
            rotation: 0.0,
            scale: 1.0 / 3.0,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 6, false)
}

fn cantor_set() -> FractalState {
    let replicas = vec![
        Replica {
            translation: Vec2::new(-2.0 / 3.0, 0.0),
            rotation: 0.0,
            scale: 1.0 / 3.0,
        },
        Replica {
            translation: Vec2::new(2.0 / 3.0, 0.0),
            rotation: 0.0,
            scale: 1.0 / 3.0,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 8, false)
}

fn vicsek() -> FractalState {
    let s = 1.0 / 3.0;
    let u = 2.0 / 3.0;
    let replicas = vec![
        Replica {
            translation: Vec2::new(-u, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(u, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(-u, u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(u, u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::ZERO,
            rotation: 0.0,
            scale: s,
        },
    ];
    state(BaseShapePreset::Square.lines(), replicas, 5, false)
}

/// Heighway dragon（発端を [-1,1] の水平線分とする標準 2 変換 IFS）。
fn heighway_dragon() -> FractalState {
    let a0 = FRAC_PI_4;
    let s = SQRT_2 / 2.0;
    let replicas = vec![
        Replica {
            translation: Vec2::new(-0.5, 0.5),
            rotation: a0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(0.5, 0.5),
            rotation: -a0,
            scale: s,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 12, false)
}

fn levy_c() -> FractalState {
    let a0 = FRAC_PI_4;
    let s = SQRT_2 / 2.0;
    let replicas = vec![
        Replica {
            translation: Vec2::new(-0.5, 0.5),
            rotation: a0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(0.5, -0.5),
            rotation: a0,
            scale: s,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 12, false)
}

/// 幹＋左右枝の相似 IFS（パラメータは見た目優先で調整）。
fn binary_tree() -> FractalState {
    let theta: f32 = 0.47;
    let s = 0.58;
    let tx = s * theta.sin();
    let ty = 1.0 - s * theta.cos();
    let replicas = vec![
        Replica {
            translation: Vec2::new(tx, ty),
            rotation: theta,
            scale: s,
        },
        Replica {
            translation: Vec2::new(-tx, ty),
            rotation: -theta,
            scale: s,
        },
    ];
    let base = vec![Line {
        a: Vec2::new(0.0, -1.0),
        b: Vec2::new(0.0, 1.0),
    }];
    state(base, replicas, 8, false)
}

fn sierpinski_carpet() -> FractalState {
    let s = 1.0 / 3.0;
    let u = 2.0 / 3.0;
    let replicas = vec![
        Replica {
            translation: Vec2::new(-u, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(0.0, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(u, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(-u, 0.0),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(u, 0.0),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(-u, u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(0.0, u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            translation: Vec2::new(u, u),
            rotation: 0.0,
            scale: s,
        },
    ];
    state(BaseShapePreset::Square.lines(), replicas, 4, false)
}

/// ピタゴラスの木: 斜辺を種とする直角三角形の 2 相似 IFS（頂点 (0,h)、h>1 で Heighway より立ち上がる）。
/// 途中世代も描くと枝分かれが読みやすい。
fn pythagoras_tree() -> FractalState {
    let h: f32 = 1.16;
    let s = (1.0 + h * h).sqrt() * 0.5;
    let cos_t = 1.0 / (2.0 * s);
    let sin_t = h / (2.0 * s);
    let theta = sin_t.atan2(cos_t);
    let replicas = vec![
        Replica {
            translation: Vec2::new(-0.5, 0.5 * h),
            rotation: theta,
            scale: s,
        },
        Replica {
            translation: Vec2::new(0.5, 0.5 * h),
            rotation: -theta,
            scale: s,
        },
    ];
    state(
        BaseShapePreset::Segment.lines(),
        replicas,
        7,
        false,
    )
}

fn sierpinski_hexagon() -> FractalState {
    let v = hexagon_vertices();
    let s = 1.0 / 3.0;
    let replicas: Vec<Replica> = v
        .into_iter()
        .map(|p| Replica {
            translation: p * (2.0 / 3.0),
            rotation: 0.0,
            scale: s,
        })
        .collect();
    state(BaseShapePreset::Hexagon.lines(), replicas, 5, false)
}
