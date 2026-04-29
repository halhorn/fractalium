// Replica fields are used in Feat2+ but some may trigger dead_code for future Feat3.
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

impl BaseShape {
    /// Axis-aligned bounding box of all line endpoints. Returns `None` when there
    /// are no lines.
    pub fn bbox(&self) -> Option<Rect> {
        let mut pts = self.lines.iter().flat_map(|l| [l.a, l.b]);
        let first = pts.next()?;
        let (min, max) = pts.fold((first, first), |(mn, mx), p| (mn.min(p), mx.max(p)));
        Some(Rect::from_corners(min, max))
    }
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
    pub scale: f32,    // must be > 0
}

impl Replica {
    pub fn default_new() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.0,
            scale: 0.5,
        }
    }

    pub fn identity() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.0,
            scale: 1.0,
        }
    }

    /// Apply scale → rotate → translate to a point in base-shape space.
    pub fn apply(&self, p: Vec2) -> Vec2 {
        let scaled = p * self.scale;
        let angle = self.rotation;
        let rotated = Vec2::new(
            scaled.x * angle.cos() - scaled.y * angle.sin(),
            scaled.x * angle.sin() + scaled.y * angle.cos(),
        );
        rotated + self.translation
    }

    /// Compose transforms: returns `c` where `c.apply(p) == self.apply(other.apply(p))`.
    pub fn compose(self, other: Replica) -> Replica {
        let cos_s = self.rotation.cos();
        let sin_s = self.rotation.sin();
        let rot_t = Vec2::new(
            other.translation.x * cos_s - other.translation.y * sin_s,
            other.translation.x * sin_s + other.translation.y * cos_s,
        );
        Replica {
            translation: self.translation + self.scale * rot_t,
            rotation: self.rotation + other.rotation,
            scale: self.scale * other.scale,
        }
    }
}

/// Undo history for discrete user actions (line confirm, clear, replica add/delete).
/// Continuous DragValue edits are not captured.
#[derive(Resource, Default)]
pub struct UndoStack {
    history: Vec<FractalState>,
}

impl UndoStack {
    pub fn push(&mut self, state: FractalState) {
        if self.history.len() >= 50 {
            self.history.remove(0);
        }
        self.history.push(state);
    }

    pub fn pop(&mut self) -> Option<FractalState> {
        self.history.pop()
    }
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
