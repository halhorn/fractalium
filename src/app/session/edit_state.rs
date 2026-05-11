//! 複数画面にまたがる編集状態
//! - Undo スタック
//! - Placement の編集中状態
//! - Seed / Placement で共有する格子吸着フラグ

use bevy::prelude::*;

use super::fractal_state::FractalState;

/// Seed / Placement で Ctrl 相当の格子吸着を有効にするか。
#[derive(Resource)]
pub struct SnapGrid(
    /// トグルオンなら Ctrl 省略時も細かい格子スナップを有効扱いにする。
    pub bool,
);

impl Default for SnapGrid {
    /// 既定はオフ。
    fn default() -> Self {
        Self(false)
    }
}

/// Placement パネルでの選択・ドラッグ状態を保持するリソース。
#[derive(Resource, Default)]
pub struct PlacementState {
    /// 選択中のレプリカインデックス。
    pub selected: Option<usize>,
    /// 現在進行中のドラッグ操作。
    pub drag: PlacementDrag,
    /// Ctrl+C でコピーしたレプリカ。
    pub clipboard: Option<crate::core::shape::Replica>,
}

/// Placement パネルのドラッグ操作の種別と開始時のスナップショット。
#[derive(Default)]
pub enum PlacementDrag {
    #[default]
    Idle,
    Move {
        start_cursor: Vec2,
        start_position: Vec2,
    },
    Scale {
        pivot: Vec2,
        start_cursor_dist: f32,
        start_scale: f32,
    },
    Rotate {
        pivot: Vec2,
        start_angle: f32,
        start_rotation: f32,
    },
    /// 空白クリック後の待機状態。一定距離動いたら Rotate に昇格。
    /// 動かずに離したら選択解除。
    RotatePending {
        pivot: Vec2,
        start_cursor: Vec2,
        start_angle: f32,
        start_rotation: f32,
    },
}

const UNDO_LIMIT: usize = 50;

/// ユーザーの離散的な操作（線確定・クリア・複製追加/削除）を巻き戻すためのスタック。
/// DragValue による連続編集は対象外。
#[derive(Resource, Default)]
pub struct UndoStack {
    history: Vec<FractalState>,
    redo_stack: Vec<FractalState>,
}

impl UndoStack {
    /// 現在の状態を履歴に積む。redo スタックはクリアされる。
    pub fn push(&mut self, state: FractalState) {
        if self.history.len() >= UNDO_LIMIT {
            self.history.remove(0);
        }
        self.history.push(state);
        self.redo_stack.clear();
    }

    /// undo: 履歴から前の状態を取り出し、current を redo スタックに積む。
    /// 履歴が空なら None を返す。
    pub fn undo_pop(&mut self, current: FractalState) -> Option<FractalState> {
        if let Some(prev) = self.history.pop() {
            self.redo_stack.push(current);
            Some(prev)
        } else {
            None
        }
    }

    /// redo: redo スタックから次の状態を取り出し、current を履歴に積む。
    /// redo スタックが空なら None を返す。
    pub fn redo_pop(&mut self, current: FractalState) -> Option<FractalState> {
        if let Some(next) = self.redo_stack.pop() {
            self.history.push(current);
            Some(next)
        } else {
            None
        }
    }

    /// Undo 可能か。
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Redo 可能か。
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// URL から状態を読み込んだときなど、履歴を空にする。
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn clear(&mut self) {
        self.history.clear();
        self.redo_stack.clear();
    }
}
