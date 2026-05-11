use std::f32::consts::FRAC_PI_3;

use bevy::prelude::Vec2;

use crate::app::session::FractalState;
use crate::core::seed_preset::BaseShapePreset;
use crate::core::shape::Replica;
use crate::fractal_presets::common::state;

/// Davis–Knuth テドラゴンの標準 3 写像 IFS（`f1(z)=z/2`, `f2(z)=e^{iπ/3}z/2+1/2`, 第三は鏡像）。
pub(super) fn build() -> FractalState {
    let half = 0.5_f32;
    let replicas = vec![
        Replica {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: half,
        },
        Replica {
            position: Vec2::new(half, 0.0),
            rotation: FRAC_PI_3,
            scale: half,
        },
        Replica {
            position: Vec2::new(half, 0.0),
            rotation: -FRAC_PI_3,
            scale: half,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 8, false)
}
