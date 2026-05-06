use crate::fractal_presets::common::{hexagon_vertices, state};
use crate::seed_shape::BaseShapePreset;
use crate::state::{FractalState, Replica};

pub(super) fn build() -> FractalState {
    let v = hexagon_vertices();
    let s = 1.0 / 3.0;
    let replicas: Vec<Replica> = v
        .into_iter()
        .map(|p| Replica {
            translation: p * (2.0 / 3.0),
            rotation: 0.0,
            scale: s,
        })
        .collect();
    state(BaseShapePreset::Hexagon.lines(), replicas, 5, false)
}
