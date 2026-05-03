//! 各キャンバスのビュー操作（ズーム・パン）を提供するモジュール。

use bevy::camera::Projection;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::{EditCamera, PlacementCamera, ResultCamera};

/// ズーム時にカーソル位置を中心にする（true）か原点固定にする（false）か。
trait ZoomTowardsCursor {
    const ZOOM_TOWARDS_CURSOR: bool;
}
impl ZoomTowardsCursor for EditCamera     { const ZOOM_TOWARDS_CURSOR: bool = false; }
impl ZoomTowardsCursor for PlacementCamera { const ZOOM_TOWARDS_CURSOR: bool = false; }
impl ZoomTowardsCursor for ResultCamera   { const ZOOM_TOWARDS_CURSOR: bool = true; }

const ZOOM_SPEED: f32 = 0.02;
const ZOOM_MIN: f32 = 0.005;
const ZOOM_MAX: f32 = 8.0;

/// Result ビューポート内での左ドラッグ状態。
#[derive(Resource, Default)]
struct PanState {
    active: bool,
    last_cursor_screen: Vec2,
}

/// 2 本指ピンチズームの追跡状態。
#[derive(Resource, Default)]
struct PinchState {
    active: bool,
    prev_distance: f32,
    prev_midpoint: Vec2,
}

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanState>()
            .init_resource::<PinchState>()
            .add_systems(
                Update,
                (
                    zoom_canvas::<EditCamera>,
                    zoom_canvas::<PlacementCamera>,
                    zoom_canvas::<ResultCamera>,
                    pan_result,
                    handle_pinch_zoom,
                ),
            );
    }
}

/// カーソルが指定カメラのビューポート内にあるか返す。
fn cursor_in_viewport(window: &Window, cam: &Camera) -> bool {
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

/// マウスホイールで任意キャンバスをズームするジェネリックシステム。
/// C::ZOOM_TOWARDS_CURSOR が true のときはカーソル位置中心、false のときは原点 (0,0) 固定。
fn zoom_canvas<C: Component + ZoomTowardsCursor>(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cam_q: Query<(&Camera, &GlobalTransform, &mut Projection, &mut Transform), With<C>>,
) {
    let total = scroll.delta.y;
    if total == 0.0 {
        return;
    }
    let Ok(window) = windows.single() else { return; };
    let Ok((cam, cam_tf, mut proj, mut transform)) = cam_q.single_mut() else { return; };

    if !cursor_in_viewport(window, cam) {
        return;
    }

    let Projection::Orthographic(ref mut ortho) = *proj else { return; };

    let old_scale = ortho.scale;
    let new_scale = (old_scale * (1.0 - total * ZOOM_SPEED)).clamp(ZOOM_MIN, ZOOM_MAX);
    let scale_ratio = new_scale / old_scale;

    if C::ZOOM_TOWARDS_CURSOR {
        // Result: カーソル下のワールド座標を固定してズーム
        if let Some(cursor_screen) = window.cursor_position() {
            if let Ok(cursor_world) = cam.viewport_to_world_2d(cam_tf, cursor_screen) {
                transform.translation.x +=
                    (cursor_world.x - transform.translation.x) * (1.0 - scale_ratio);
                transform.translation.y +=
                    (cursor_world.y - transform.translation.y) * (1.0 - scale_ratio);
            }
        }
    } else {
        // Edit / Placement: 原点 (0,0) を中心にズーム
        transform.translation.x *= scale_ratio;
        transform.translation.y *= scale_ratio;
    }

    ortho.scale = new_scale;
}

/// 左ドラッグで Result カメラをパン（平行移動）する。
fn pan_result(
    mut pan_state: ResMut<PanState>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut result_cam_q: Query<(&Camera, &GlobalTransform, &mut Transform), With<ResultCamera>>,
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

    if buttons.just_pressed(MouseButton::Left) && cursor_in_viewport(window, cam) {
        pan_state.active = true;
        pan_state.last_cursor_screen = cursor_screen;
    }

    if pan_state.active && buttons.pressed(MouseButton::Left) {
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

/// 2 本指ピンチで Result カメラをズームする。
fn handle_pinch_zoom(
    touches: Res<Touches>,
    mut pinch: ResMut<PinchState>,
    mut result_cam_q: Query<(&Camera, &GlobalTransform, &mut Projection, &mut Transform), With<ResultCamera>>,
) {
    let positions: Vec<Vec2> = touches.iter().map(|t| t.position()).collect();

    if positions.len() == 2 {
        let distance = (positions[1] - positions[0]).length();
        let midpoint = (positions[0] + positions[1]) * 0.5;

        if pinch.active && pinch.prev_distance > 0.0 {
            let Ok((cam, cam_tf, mut proj, mut transform)) = result_cam_q.single_mut() else {
                pinch.prev_distance = distance;
                pinch.prev_midpoint = midpoint;
                return;
            };
            if let Projection::Orthographic(ref mut ortho) = *proj {
                let scale_factor = pinch.prev_distance / distance;
                let old_scale = ortho.scale;
                let new_scale = (old_scale * scale_factor).clamp(ZOOM_MIN, ZOOM_MAX);
                let scale_ratio = new_scale / old_scale;

                if let Ok(mid_world) = cam.viewport_to_world_2d(cam_tf, midpoint) {
                    transform.translation.x += (mid_world.x - transform.translation.x) * (1.0 - scale_ratio);
                    transform.translation.y += (mid_world.y - transform.translation.y) * (1.0 - scale_ratio);
                }
                ortho.scale = new_scale;
            }
        }

        pinch.active = true;
        pinch.prev_distance = distance;
        pinch.prev_midpoint = midpoint;
    } else {
        pinch.active = false;
        pinch.prev_distance = 0.0;
    }
}
