use bevy::prelude::Vec2;

use crate::app::session::FractalState;
use crate::core::seed_preset::BaseShapePreset;
use crate::core::shape::Replica;
use crate::fractal_presets::common::state;

pub(super) fn build() -> FractalState {
    let s = 1.0 / 3.0;
    let u = 2.0 / 3.0;
    let replicas = vec![
        Replica {
            position: Vec2::new(-u, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            position: Vec2::new(0.0, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            position: Vec2::new(u, -u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            position: Vec2::new(-u, 0.0),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            position: Vec2::new(u, 0.0),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            position: Vec2::new(-u, u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            position: Vec2::new(0.0, u),
            rotation: 0.0,
            scale: s,
        },
        Replica {
            position: Vec2::new(u, u),
            rotation: 0.0,
            scale: s,
        },
    ];
    state(BaseShapePreset::Square.lines(), replicas, 4, false)
}
