use crate::fractal_presets::common::{state, triangle_vertices};
use crate::seed_shape::BaseShapePreset;
use crate::state::{FractalState, Replica};

pub(super) fn build() -> FractalState {
    let replicas: Vec<Replica> = triangle_vertices()
        .into_iter()
        .map(|p| Replica {
            translation: p * 0.5,
            rotation: 0.0,
            scale: 0.5,
        })
        .collect();
    state(BaseShapePreset::Triangle.lines(), replicas, 6, false)
}
