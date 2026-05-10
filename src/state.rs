use bevy::prelude::*;

pub use crate::core::{
    BaseShape, FRACTAL_DEPTH_HARD_CAP, Line, REPLICA_SCALE_MAX, REPLICA_SCALE_MIN, Replica,
};

/// フラクタル全体の状態を表す Bevy リソース。
/// 「基図形 → 複製ルール → 再帰深さ」の 3 要素でフラクタルが一意に決まる。
/// 座標は正規化キャンバス座標 [-1, 1] x [-1, 1] を用いる。
#[derive(Resource, Clone)]
pub struct FractalState {
    /// 再帰の元となる基図形（線分の集合）
    pub base_shape: BaseShape,
    /// 再帰時に基図形を配置する複製変換のリスト
    pub replicas: Vec<Replica>,
    /// 再帰の深さ（1 = 基図形のみ、2 以上で replicas が適用される）
    pub depth: u32,
    /// true のとき、末端世代だけでなく途中世代の図形も描画する
    pub show_all_generations: bool,
    /// グリッドスナップ（Ctrl 相当）のトグル状態。
    pub snap_grid: bool,
}

impl Default for FractalState {
    /// 初期状態：基図形・複製ともに空、深さは 4。
    fn default() -> Self {
        Self {
            base_shape: BaseShape::default(),
            replicas: vec![],
            depth: 4,
            show_all_generations: false,
            snap_grid: false,
        }
    }
}

/// 論理ピクセル（左上原点・Y 下向き）の軸平行矩形。ウィンドウ座標と egui と揃える。
#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl ScreenRect {
    pub fn contains(&self, pos: Vec2) -> bool {
        pos.x >= self.min.x && pos.x <= self.max.x && pos.y >= self.min.y && pos.y <= self.max.y
    }
}

/// Placement キャンバスの論理ピクセル矩形（egui オーバーレイの位置合わせに使用）。
#[derive(Resource, Default)]
pub struct CanvasLayout {
    pub placement_min_x: f32,
    pub placement_max_x: f32,
    pub placement_min_y: f32,
    pub placement_max_y: f32,
    /// Result に重ねている Depth／Show generations の描画矩形。None のときビュー入力は矩形で止めない。
    pub result_depth_controls_rect: Option<ScreenRect>,
}

/// ダブルタップドラッグによるズームが進行中かどうかを示すフラグ。
/// edit / placement システムがタッチ入力を無視するために参照する。
#[derive(Resource, Default)]
pub struct DoubleTapZoomActive(pub bool);

/// URL からの復元やフルプリセット適用後、Result カメラを図形に合わせる。
#[derive(Resource, Default)]
pub struct PendingResultCameraFit(pub bool);

/// UI レイアウト状態（パネルの折りたたみなど）を保持するリソース。
#[derive(Resource)]
pub struct UiLayout {
    pub params_collapsed: bool,
}

impl Default for UiLayout {
    fn default() -> Self {
        Self {
            params_collapsed: true,
        }
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
    pub clipboard: Option<Replica>,
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

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }
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
