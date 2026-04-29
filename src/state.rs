// Fields populated and consumed by upcoming Feat1 (line drawing),
// Feat2 (replica editing), Feat3 (recursive render). Allow until then.
#![allow(dead_code)]

use bevy::prelude::*;

/// Single source of truth for the fractal: the base shape that gets recursively
/// replicated, the placement rules for those replicas, and how many recursive
/// levels to render. All positions are in normalized canvas coordinates
/// ([-1, 1] x [-1, 1]).
#[derive(Resource, Default, Clone)]
pub struct FractalState {
    pub base_shape: BaseShape,
    pub replicas: Vec<Replica>,
    pub depth: u32,
}

#[derive(Default, Clone)]
pub struct BaseShape {
    pub lines: Vec<Line>,
}

#[derive(Clone, Copy)]
pub struct Line {
    pub a: Vec2,
    pub b: Vec2,
}

/// Affine placement of one replica of the base shape: translation, rotation,
/// uniform scale. Order of application is scale → rotate → translate.
#[derive(Clone, Copy)]
pub struct Replica {
    pub translation: Vec2,
    pub rotation: f32, // radians
    pub scale: f32,
}

impl FractalState {
    pub fn default_mvp() -> Self {
        Self {
            base_shape: BaseShape::default(),
            replicas: vec![],
            depth: 4,
        }
    }
}
