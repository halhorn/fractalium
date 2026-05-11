//! hal V Star フラクタル全体プリセット。

use bevy::prelude::Vec2;

use crate::app::session::FractalState;
use crate::core::shape::{Line, Replica};
use crate::fractal_presets::common::state;

/// hal V Star: 短い縦基線と 3 複製。
pub(super) fn build() -> FractalState {
    let base_shape = vec![Line {
        a: Vec2::new(0.0, -0.25),
        b: Vec2::new(0.0, 0.75),
    }];
    let replicas = vec![
        Replica {
            position: Vec2::new(0.0, -0.375),
            rotation: 0.0,
            scale: 0.5,
        },
        Replica {
            position: Vec2::new(0.375, -0.25),
            rotation: -7.85398,
            scale: 0.5,
        },
        Replica {
            position: Vec2::new(-0.375, -0.25),
            rotation: 1.5708,
            scale: 0.5,
        },
    ];
    state(base_shape, replicas, 10, false)
}
