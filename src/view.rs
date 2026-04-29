//! Result キャンバスのビュー操作（マウスホイールズーム）を提供するモジュール。

use bevy::camera::Projection;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::ResultCamera;

/// ホイール 1 ノッチあたりのズーム係数。
const ZOOM_SPEED: f32 = 0.1;
/// ズーム後の最小・最大スケール。
const ZOOM_MIN: f32 = 0.2;
const ZOOM_MAX: f32 = 8.0;

/// Result カメラへのズーム入力を扱うプラグイン。
pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, zoom_result);
    }
}

/// マウスホイールの動きを Result カメラの直交投影スケールに反映する。
/// カーソルが Result ビューポート外にあるときは何もしない。
fn zoom_result(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut result_cam_q: Query<(&Camera, &GlobalTransform, &mut Projection), With<ResultCamera>>,
) {
    let total = scroll.delta.y;
    if total == 0.0 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((cam, cam_tf, mut proj)) = result_cam_q.single_mut() else {
        return;
    };

    let cursor_in_result = window
        .cursor_position()
        .and_then(|p| cam.viewport_to_world_2d(cam_tf, p).ok())
        .is_some();
    if !cursor_in_result {
        return;
    }

    if let Projection::Orthographic(ref mut ortho) = *proj {
        ortho.scale = (ortho.scale * (1.0 - total * ZOOM_SPEED)).clamp(ZOOM_MIN, ZOOM_MAX);
    }
}
