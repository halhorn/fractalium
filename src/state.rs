use bevy::prelude::*;

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
}

impl Default for FractalState {
    /// 初期状態：基図形・複製ともに空、深さは 4。
    fn default() -> Self {
        Self {
            base_shape: BaseShape::default(),
            replicas: vec![],
            depth: 4,
        }
    }
}

/// 再帰の元になる基図形。現状は線分の集合のみで表現する。
#[derive(Default, Clone)]
pub struct BaseShape {
    pub lines: Vec<Line>,
}

/// 2 点で表される 2D 線分。
#[derive(Clone, Copy)]
pub struct Line {
    pub a: Vec2,
    pub b: Vec2,
}

/// 1 つの複製を表す相似変換（平行移動・回転・一様スケール）。
/// 適用順は scale → rotate → translate。
#[derive(Clone, Copy)]
pub struct Replica {
    /// 平行移動量
    pub translation: Vec2,
    /// 回転角（ラジアン）
    pub rotation: f32,
    /// 一様スケール（> 0）
    pub scale: f32,
}

impl Replica {
    /// UI から複製が新規追加されたときの初期値。原点・無回転・半分スケール。
    pub fn default_new() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.0,
            scale: 0.5,
        }
    }

    /// 何も変換しない恒等変換。再帰描画の起点として用いる。
    pub fn identity() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.0,
            scale: 1.0,
        }
    }

    /// 基図形空間の点 `p` に scale → rotate → translate を適用する。
    pub fn apply(&self, p: Vec2) -> Vec2 {
        let scaled = p * self.scale;
        let (sin, cos) = self.rotation.sin_cos();
        Vec2::new(scaled.x * cos - scaled.y * sin, scaled.x * sin + scaled.y * cos)
            + self.translation
    }

    /// 2 つの相似変換を合成する。
    /// 戻り値 `c` は `c.apply(p) == self.apply(other.apply(p))` を満たす。
    pub fn compose(self, other: Replica) -> Replica {
        let (sin_s, cos_s) = self.rotation.sin_cos();
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

/// 各パネルの論理ピクセルサイズを保持するリソース（egui オーバーレイの位置合わせに使用）。
#[derive(Resource, Default)]
pub struct CanvasLayout {
    /// Edit / Placement パネルの幅（論理ピクセル）。
    pub left_w_logical: f32,
    /// Edit パネルの高さ（＝ Placement パネルの上端、論理ピクセル）。
    pub top_h_logical: f32,
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
    /// ダブルクリック検出用の前回クリック時刻（秒）。
    pub last_click_time: f64,
    /// ダブルクリック検出用の前回クリック位置（ワールド座標）。
    pub last_click_pos: Vec2,
}

/// Placement パネルのドラッグ操作の種別と開始時のスナップショット。
#[derive(Default)]
pub enum PlacementDrag {
    #[default]
    Idle,
    Move {
        start_cursor: Vec2,
        start_translation: Vec2,
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
}

const UNDO_LIMIT: usize = 50;

/// ユーザーの離散的な操作（線確定・クリア・複製追加/削除）を巻き戻すためのスタック。
/// DragValue による連続編集は対象外。
#[derive(Resource, Default)]
pub struct UndoStack {
    history: Vec<FractalState>,
}

impl UndoStack {
    /// 現在の状態を履歴に積む。上限に達したら最古の履歴を破棄する。
    pub fn push(&mut self, state: FractalState) {
        if self.history.len() >= UNDO_LIMIT {
            self.history.remove(0);
        }
        self.history.push(state);
    }

    /// 最後に積んだ状態を取り出す。履歴が空なら `None`。
    pub fn pop(&mut self) -> Option<FractalState> {
        self.history.pop()
    }
}
