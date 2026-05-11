use crate::app::session::FractalState;
use crate::core::seed_preset::BaseShapePreset;
use crate::core::shape::Replica;
use crate::fractal_presets::common::{hexagon_vertices, state};

pub(super) fn build() -> FractalState {
    let v = hexagon_vertices();
    let s = 1.0 / 3.0;
    let replicas: Vec<Replica> = v
        .into_iter()
        .map(|p| Replica {
            position: p * (2.0 / 3.0),
            rotation: 0.0,
            scale: s,
        })
        .collect();
    state(BaseShapePreset::Hexagon.lines(), replicas, 5, false)
}
