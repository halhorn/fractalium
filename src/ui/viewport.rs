//! メイン UI パスで決まった egui 領域を、Edit / Placement / Result の各 [`Camera`] ビューポートに写す。

use bevy::camera::Viewport;
use bevy::ecs::system::SystemParam;
use bevy::prelude::{Camera, Query, UVec2, With, Without, default};
use bevy_egui::egui;

use crate::{EditCamera, PlacementCamera, ResultCamera};

/// `params_panel` のシステム引数個数制限を避けるため、3 つのカメラ用 [`Query`] を 1 つの [`SystemParam`] に束ねる。
#[derive(SystemParam)]
#[allow(clippy::type_complexity)] // 排他 With / Without フィルタ付き Query のため型が長い。
pub(crate) struct ViewportCamerasMut<'w, 's> {
    /// Edit（Seed）キャンバスを描画するカメラ。
    pub(crate) edit_cam: Query<
        'w,
        's,
        &'static mut Camera,
        (
            With<EditCamera>,
            Without<PlacementCamera>,
            Without<ResultCamera>,
        ),
    >,
    /// Placement キャンバスを描画するカメラ。
    pub(crate) placement_cam: Query<
        'w,
        's,
        &'static mut Camera,
        (
            With<PlacementCamera>,
            Without<EditCamera>,
            Without<ResultCamera>,
        ),
    >,
    /// Result（再帰図形）を描画するカメラ。
    pub(crate) result_cam: Query<
        'w,
        's,
        &'static mut Camera,
        (
            With<ResultCamera>,
            Without<EditCamera>,
            Without<PlacementCamera>,
        ),
    >,
}

/// `rect` をウィンドウ物理ピクセル座標へ写し、はみ出しを切り詰めた [`Viewport`] を返す。
///
/// `scale` はウィンドウの `scale_factor`、`win_phys` は `physical_width` × `physical_height`。
/// 矩形が無効なら `None`。
pub(crate) fn egui_rect_to_viewport(rect: egui::Rect, scale: f32, win_phys: UVec2) -> Option<Viewport> {
    let x = ((rect.min.x * scale).round() as i32).max(0) as u32;
    let y = ((rect.min.y * scale).round() as i32).max(0) as u32;
    let w = ((rect.width() * scale).round() as i32).max(0) as u32;
    let h = ((rect.height() * scale).round() as i32).max(0) as u32;
    if x >= win_phys.x || y >= win_phys.y || w == 0 || h == 0 {
        return None;
    }
    let w = w.min(win_phys.x - x);
    let h = h.min(win_phys.y - y);
    if w == 0 || h == 0 {
        return None;
    }
    Some(Viewport {
        physical_position: UVec2::new(x, y),
        physical_size: UVec2::new(w, h),
        ..default()
    })
}
