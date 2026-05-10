use std::f32::consts::{FRAC_PI_4, SQRT_2};

use bevy::prelude::Vec2;

use crate::fractal_presets::common::state;
use crate::seed_shape::BaseShapePreset;
use crate::state::{FractalState, Replica};

/// 対称ピタゴラスの木: 正方形の上辺を斜辺とする等辺直角三角形の二脚に、
/// 辺長 1/√2 の子正方形を外向きに載せる（左 +45°・右 −45°）。`Square` と同じ
/// 頂点（±1）を持つ基図形で再帰する。
/// 既定は深さ 12・全世代表示で各オーダーを重ねて見えるようにする。
pub(super) fn build() -> FractalState {
    let s = SQRT_2 / 2.0;
    let replicas = vec![
        Replica {
            position: Vec2::new(-1.0, 2.0),
            rotation: FRAC_PI_4,
            scale: s,
        },
        Replica {
            position: Vec2::new(1.0, 2.0),
            rotation: -FRAC_PI_4,
            scale: s,
        },
    ];
    state(BaseShapePreset::Square.lines(), replicas, 12, true)
}
