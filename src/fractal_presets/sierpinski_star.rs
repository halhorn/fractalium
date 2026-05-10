use crate::fractal_presets::common::{pentagon_vertices, state};
use crate::seed_shape::BaseShapePreset;
use crate::state::{FractalState, Replica};

/// 正五角星（ペンタグラム）の各先端方向へ 1/3 縮小。
pub(super) fn build() -> FractalState {
    let v = pentagon_vertices();
    let s = 1.0 / 3.0;
    let replicas: Vec<Replica> = v
        .into_iter()
        .map(|p| Replica {
            position: p * (2.0 / 3.0),
            rotation: 0.0,
            scale: s,
        })
        .collect();
    state(BaseShapePreset::Star.lines(), replicas, 5, false)
}
