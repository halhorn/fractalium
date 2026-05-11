//! hal Wing フラクタル全体プリセット。

use bevy::prelude::Vec2;

use crate::app::session::FractalState;
use crate::core::shape::{Line, Replica};
use crate::fractal_presets::common::state;

/// hal Wing: 水平直径の基線と 5 複製。
pub(super) fn build() -> FractalState {
    let base_shape = vec![Line {
        a: Vec2::new(-1.0, 0.0),
        b: Vec2::new(1.0, 0.0),
    }];
    let replicas = vec![
        Replica {
            position: Vec2::new(-0.666667, 0.0),
            rotation: 0.0,
            scale: 0.333333,
        },
        Replica {
            position: Vec2::new(-0.166667, 0.288675),
            rotation: 1.0472,
            scale: 0.333333,
        },
        Replica {
            position: Vec2::new(0.166667, 0.288675),
            rotation: -1.0472,
            scale: 0.333333,
        },
        Replica {
            position: Vec2::new(0.666667, 0.0),
            rotation: 0.0,
            scale: 0.333333,
        },
        Replica {
            position: Vec2::new(0.0, -0.382463),
            rotation: 0.0,
            scale: 0.33773,
        },
    ];
    state(base_shape, replicas, 7, false)
}
