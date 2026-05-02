//! Result キャンバスのビュー操作（ズーム・パン）を提供するモジュール。

use bevy::camera::Projection;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::ResultCamera;

/// ホイール 1 ノッチあたりのズーム係数（小さいほど細かい）。
const ZOOM_SPEED: f32 = 0.02;
const ZOOM_MIN: f32 = 0.005;
const ZOOM_MAX: f32 = 8.0;

/// Result ビューポート内での左ドラッグ状態。
#[derive(Resource, Default)]
struct PanState {
    active: bool,
    last_cursor_screen: Vec2,
}

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanState>()
            .add_systems(Update, (zoom_result, pan_result));
    }
}

/// カーソルが Result カメラのビューポート内にあるか返す。
fn cursor_in_result_viewport(window: &Window, cam: &Camera) -> bool {
    let Some(cursor) = window.cursor_position() else { return false; };
    if let Some(ref vp) = cam.viewport {
        let scale = window.scale_factor();
        let vp_min = vp.physical_position.as_vec2() / scale;
        let vp_size = vp.physical_size.as_vec2() / scale;
        cursor.x >= vp_min.x
            && cursor.y >= vp_min.y
            && cursor.x <= vp_min.x + vp_size.x
            && cursor.y <= vp_min.y + vp_size.y
    } else {
        true
    }
}

/// マウスホイールで Result カメラをズームする。カーソル位置を中心に拡縮する。
fn zoom_result(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut result_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Projection, &mut Transform),
        With<ResultCamera>,
    >,
) {
    let total = scroll.delta.y;
    if total == 0.0 {
        return;
    }
    let Ok(window) = windows.single() else { return; };
    let Ok((cam, cam_tf, mut proj, mut transform)) = result_cam_q.single_mut() else { return; };

    if !cursor_in_result_viewport(window, cam) {
        return;
    }

    let Projection::Orthographic(ref mut ortho) = *proj else { return; };

    let old_scale = ortho.scale;
    let new_scale = (old_scale * (1.0 - total * ZOOM_SPEED)).clamp(ZOOM_MIN, ZOOM_MAX);

    // カーソルのワールド座標を基準にズームし、カーソル下の点が変わらないよう補正する
    if let Some(cursor_screen) = window.cursor_position() {
        if let Ok(cursor_world) = cam.viewport_to_world_2d(cam_tf, cursor_screen) {
            let scale_ratio = new_scale / old_scale;
            transform.translation.x +=
                (cursor_world.x - transform.translation.x) * (1.0 - scale_ratio);
            transform.translation.y +=
                (cursor_world.y - transform.translation.y) * (1.0 - scale_ratio);
        }
    }

    ortho.scale = new_scale;
}

/// 左ドラッグで Result カメラをパン（平行移動）する。
fn pan_result(
    mut pan_state: ResMut<PanState>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut result_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Transform),
        With<ResultCamera>,
    >,
) {
    let Ok(window) = windows.single() else { return; };
    let Ok((cam, cam_tf, mut transform)) = result_cam_q.single_mut() else { return; };

    let Some(cursor_screen) = window.cursor_position() else {
        pan_state.active = false;
        return;
    };

    if buttons.just_released(MouseButton::Left) {
        pan_state.active = false;
    }

    if buttons.just_pressed(MouseButton::Left) && cursor_in_result_viewport(window, cam) {
        pan_state.active = true;
        pan_state.last_cursor_screen = cursor_screen;
    }

    if pan_state.active && buttons.pressed(MouseButton::Left) {
        // フレーム間のカーソル移動量をワールド座標で求め、カメラを逆方向に動かす
        if let (Ok(prev_world), Ok(curr_world)) = (
            cam.viewport_to_world_2d(cam_tf, pan_state.last_cursor_screen),
            cam.viewport_to_world_2d(cam_tf, cursor_screen),
        ) {
            let delta = curr_world - prev_world;
            transform.translation.x -= delta.x;
            transform.translation.y -= delta.y;
        }
        pan_state.last_cursor_screen = cursor_screen;
    }
}
