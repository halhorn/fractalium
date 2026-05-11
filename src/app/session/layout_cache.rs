//! 画面上の矩形キャッシュ（キャンバス配置・ヒットテスト用）。`FractalState` から導かれる不変ではない。

use bevy::prelude::*;

/// 論理ピクセル（左上原点・Y 下向き）の軸平行矩形。ウィンドウ座標と egui と揃える。
#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl ScreenRect {
    /// `pos` が矩形内なら true。
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
