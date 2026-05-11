//! hal Tree フラクタル全体プリセット。

use bevy::prelude::Vec2;

use crate::app::session::FractalState;
use crate::core::shape::{Line, Replica};
use crate::fractal_presets::common::state;

/// hal Tree: 縦長基線と 3 複製、深さ 10・全世代表示。
pub(super) fn build() -> FractalState {
    let base_shape = vec![Line {
        a: Vec2::new(0.0, 1.0),
        b: Vec2::new(0.0, -1.0),
    }];
    let replicas = vec![
        Replica {
            position: Vec2::new(0.313794, 1.05748),
            rotation: -0.578896,
            scale: 0.573507,
        },
        Replica {
            position: Vec2::new(-0.390499, 0.447508),
            rotation: 0.692576,
            scale: 0.589225,
        },
        Replica {
            position: Vec2::new(0.296448, 0.0342055),
            rotation: -0.634031,
            scale: 0.5,
        },
    ];
    state(base_shape, replicas, 10, true)
}
