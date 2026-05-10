use std::f32::consts::{FRAC_PI_4, SQRT_2};

use bevy::prelude::Vec2;

use crate::fractal_presets::common::state;
use crate::core::seed_preset::BaseShapePreset;
use crate::core::shape::Replica;
use crate::state::FractalState;

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
            position: Vec2::new(0.5, -0.5),
            rotation: a0,
            scale: s,
        },
    ];
    state(BaseShapePreset::Segment.lines(), replicas, 12, false)
}
