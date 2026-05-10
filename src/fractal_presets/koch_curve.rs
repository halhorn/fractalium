use std::f32::consts::FRAC_PI_3;

use bevy::prelude::Vec2;

use crate::fractal_presets::common::state;
use crate::core::seed_preset::BaseShapePreset;
use crate::core::shape::Replica;
use crate::state::FractalState;

/// Koch 曲線: [-1,1] の線分に 4 つの相似（回転込み）。
pub(super) fn build() -> FractalState {
    let sqrt3_6 = FRAC_PI_3.sin() / 3.0;
    let replicas = vec![
        Replica {
            position: Vec2::new(-2.0 / 3.0, 0.0),
            rotation: 0.0,
            scale: 1.0 / 3.0,
        },
        Replica {
            position: Vec2::new(-1.0 / 6.0, sqrt3_6),
            rotation: FRAC_PI_3,
            scale: 1.0 / 3.0,
        },
        Replica {
            position: Vec2::new(1.0 / 6.0, sqrt3_6),
            rotation: -FRAC_PI_3,
            scale: 1.0 / 3.0,
        },
        Replica {
            position: Vec2::new(2.0 / 3.0, 0.0),
            rotation: 0.0,
            scale: 1.0 / 3.0,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 6, false)
}
