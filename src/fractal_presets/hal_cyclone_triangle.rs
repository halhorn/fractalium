//! hal Cyclone Triangle フラクタル全体プリセット。

use bevy::prelude::Vec2;

use crate::app::session::FractalState;
use crate::core::shape::{Line, Replica};
use crate::fractal_presets::common::state;

/// hal Cyclone Triangle: 垂直基線と 2 複製、深さ 15・全世代表示。
pub(super) fn build() -> FractalState {
    let base_shape = vec![Line {
        a: Vec2::new(0.0, 0.0),
        b: Vec2::new(0.0, 1.0),
    }];
    let replicas = vec![
        Replica {
            position: Vec2::new(0.0, 1.0),
            rotation: -1.9326,
            scale: 0.707107,
        },
        Replica {
            position: Vec2::new(0.0, 0.0),
            rotation: 0.0,
            scale: 0.5,
        },
    ];
    state(base_shape, replicas, 15, true)
}
