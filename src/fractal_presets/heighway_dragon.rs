use std::f32::consts::{FRAC_PI_4, SQRT_2};

use bevy::prelude::Vec2;

use crate::fractal_presets::common::state;
use crate::seed_shape::BaseShapePreset;
use crate::state::{FractalState, Replica};

/// Heighway dragon（発端を [-1,1] の水平線分とする 2 変換 IFS）。
/// ウィキペディア等の [0,1]×R 側の基底式と、[-1,1] の種を結ぶ Φ(x,y)=(2x-1,2y)
/// としたうえで Phi o f_i = Replica_i o Phi となる Replica。[f_1 は +45 度、f_2 は +135 度]
pub(super) fn build() -> FractalState {
    let a0 = FRAC_PI_4;
    let s = SQRT_2 / 2.0;
    let replicas = vec![
        Replica {
            position: Vec2::new(-0.5, 0.5),
            rotation: a0,
            scale: s,
        },
        Replica {
            position: Vec2::new(0.5, 0.5),
            rotation: 3.0 * a0,
            scale: s,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 12, false)
}
