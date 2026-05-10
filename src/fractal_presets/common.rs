//! プリセット間で共有する `FractalState` 構築ヘルパと座変換。

use bevy::prelude::Vec2;

use crate::core::seed_preset::BaseShapePreset;
use crate::core::shape::{BaseShape, Line, Replica};
use crate::state::FractalState;

/// `snap_grid` は呼び出し側で引き継ぐ。ここでは `false` で埋めておく。
pub(super) fn state(
    base_shape: Vec<Line>,
    replicas: Vec<Replica>,
    depth: u32,
    show_all_generations: bool,
) -> FractalState {
    FractalState {
        base_shape: BaseShape { lines: base_shape },
        replicas,
        depth,
        show_all_generations,
        snap_grid: false,
    }
}

/// `Replica::apply` と同じ定義（反時計回りが正）で -90°。
#[inline]
pub(super) fn rotate_neg_90(v: Vec2) -> Vec2 {
    Vec2::new(v.y, -v.x)
}

#[inline]
pub(super) fn flip_y(v: Vec2) -> Vec2 {
    Vec2::new(v.x, -v.y)
}

/// `BaseShapePreset::Triangle` と同じ 3 頂点（線分の始点を順に繋ぐ）。
pub(super) fn triangle_vertices() -> [Vec2; 3] {
    let tri = BaseShapePreset::Triangle.lines();
    [tri[0].a, tri[1].a, tri[2].a]
}

pub(super) fn hexagon_vertices() -> Vec<Vec2> {
    BaseShapePreset::Hexagon
        .lines()
        .iter()
        .map(|l| l.a)
        .collect()
}

pub(super) fn pentagon_vertices() -> Vec<Vec2> {
    BaseShapePreset::Pentagon
        .lines()
        .iter()
        .map(|l| l.a)
        .collect()
}
