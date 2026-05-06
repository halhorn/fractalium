//! 各キャンバスのビュー操作（ズーム・パン）を提供するモジュール。

use bevy::camera::Projection;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::fractal::fractal_world_aabb;
use crate::state::{
    CanvasLayout, DoubleTapZoomActive, FractalState, PendingResultCameraFit, PlacementState,
    Replica, REPLICA_SCALE_MAX, REPLICA_SCALE_MIN,
};
use crate::{EditCamera, PlacementCamera, ResultCamera};

/// 論理画面上の位置が Depth／generations オーバーレイ上か。
#[inline]
fn pos_on_result_depth_overlay(layout: &CanvasLayout, pos: Vec2) -> bool {
    layout
        .result_depth_controls_rect
        .is_some_and(|r| r.contains(pos))
}

/// ズーム時にカーソル位置を中心にする（true）か原点固定にする（false）か。
/// Result のみ、`USE_RESULT_DEPTH_BLOCK` でオーバーレイ直上のホイールを無視する。
trait ZoomTowardsCursor {
    const ZOOM_TOWARDS_CURSOR: bool;
    const USE_RESULT_DEPTH_BLOCK: bool;
}
impl ZoomTowardsCursor for EditCamera {
    const ZOOM_TOWARDS_CURSOR: bool = false;
    const USE_RESULT_DEPTH_BLOCK: bool = false;
}
impl ZoomTowardsCursor for ResultCamera {
    const ZOOM_TOWARDS_CURSOR: bool = true;
    const USE_RESULT_DEPTH_BLOCK: bool = true;
}

const ZOOM_SPEED: f32 = 0.02;
const ZOOM_MIN: f32 = 0.005;
const ZOOM_MAX: f32 = 8.0;

/// `main::normalized_projection` の `AutoMin` 値と一致させる（Result カメラの見える範囲計算用）。
const RESULT_ORTHO_AUTOMIN: f32 = 2.0;
/// 図形全体が入る最大ズームに対して、周囲に空ける量（境界の長辺に対する比率）。
const RESULT_FIT_PADDING_RATIO: f32 = 0.06;
/// Result 左上の Depth / Show generations オーバーレイに合わせ、フィット計算で使う有効高さから控える量（論理 px）。
const RESULT_FIT_DEPTH_OVERLAY_STRIP_PX: f32 = 52.0;
/// オーバーレイ直下に余白を残すため、図形中心を画面やや下へずらす（1 画面あたりのワールド高さに対する比率）。
const RESULT_FIT_TOP_SCREEN_BIAS_FRAC: f32 = 0.035;

/// ダブルタップ判定の最大時間間隔（秒）。
const DOUBLE_TAP_TIME: f64 = 0.35;
/// ダブルタップ判定の最大指移動距離（論理ピクセル）。
const DOUBLE_TAP_DIST: f32 = 60.0;
/// ダブルタップドラッグのズーム速度（delta_y px あたりのスケール変化率）。
const DOUBLE_TAP_ZOOM_SPEED: f32 = 0.005;

/// Result ビューポート内でのパン状態（マウス + タッチ）。
#[derive(Resource, Default)]
struct PanState {
    mouse_active: bool,
    mouse_last_pos: Vec2,
    touch_id: Option<u64>,
    touch_last_pos: Vec2,
}

/// ピンチ操作の対象カメラ。
#[derive(Default, Clone, Copy, PartialEq)]
enum PinchTarget { #[default] None, Edit, Placement, Result }

/// 2 本指ピンチズームの追跡状態。
#[derive(Resource, Default)]
struct PinchState {
    active: bool,
    prev_distance: f32,
    prev_midpoint: Vec2,
    target: PinchTarget,
}

/// ダブルタップ後にドラッグしてズームするジェスチャーの追跡状態。
#[derive(Resource, Default)]
struct DoubleTapZoomState {
    /// 前回タップの時刻（秒）。
    last_tap_time: f64,
    /// 前回タップのスクリーン座標。
    last_tap_pos: Vec2,
    /// ドラッグ中のタッチ ID（None = 非アクティブ）。
    active_id: Option<u64>,
    /// ズーム対象のカメラ。
    active_target: PinchTarget,
    /// 前フレームのスクリーン Y 座標。
    last_y: f32,
}

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanState>()
            .init_resource::<PinchState>()
            .init_resource::<DoubleTapZoomState>()
            .init_resource::<DoubleTapZoomActive>()
            .add_systems(
                Update,
                (
                    zoom_canvas::<EditCamera>,
                    zoom_placement_canvas,
                    zoom_canvas::<ResultCamera>,
                    handle_pinch_zoom,
                    handle_double_tap_zoom.after(handle_pinch_zoom),
                    pan_result.after(handle_double_tap_zoom),
                ),
            );
    }
}

/// 任意の論理座標がカメラのビューポート内にあるか返す。
fn pos_in_viewport(pos: Vec2, window: &Window, cam: &Camera) -> bool {
    if let Some(ref vp) = cam.viewport {
        let scale = window.scale_factor();
        let vp_min = vp.physical_position.as_vec2() / scale;
        let vp_size = vp.physical_size.as_vec2() / scale;
        pos.x >= vp_min.x
            && pos.y >= vp_min.y
            && pos.x <= vp_min.x + vp_size.x
            && pos.y <= vp_min.y + vp_size.y
    } else {
        true
    }
}

/// カーソルが指定カメラのビューポート内にあるか返す。
fn cursor_in_viewport(window: &Window, cam: &Camera) -> bool {
    window.cursor_position().is_some_and(|pos| pos_in_viewport(pos, window, cam))
}

fn apply_replica_scale_factor(replica: &mut Replica, factor: f32) {
    replica.scale = (replica.scale * factor).clamp(REPLICA_SCALE_MIN, REPLICA_SCALE_MAX);
}

/// Placement パネル: レプリカ選択中はホイールでカメラではなく選択レプリカのスケールを変える。
fn zoom_placement_canvas(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    placement: Res<PlacementState>,
    mut state: ResMut<FractalState>,
    mut cam_q: Query<(&Camera, &GlobalTransform, &mut Projection, &mut Transform), With<PlacementCamera>>,
) {
    let total = scroll.delta.y;
    if total == 0.0 {
        return;
    }
    let Ok(window) = windows.single() else { return; };
    let Ok((cam, _, mut proj, mut transform)) = cam_q.single_mut() else { return; };

    if !cursor_in_viewport(window, cam) {
        return;
    }

    let Projection::Orthographic(ref mut ortho) = *proj else { return; };

    let old_scale = ortho.scale;
    let new_scale = (old_scale * (1.0 - total * ZOOM_SPEED)).clamp(ZOOM_MIN, ZOOM_MAX);
    let scale_ratio = new_scale / old_scale;

    if let Some(sel) = placement.selected {
        if sel < state.replicas.len() {
            if let Some(replica) = state.replicas.get_mut(sel) {
                apply_replica_scale_factor(replica, scale_ratio);
            }
            return;
        }
    }

    transform.translation.x *= scale_ratio;
    transform.translation.y *= scale_ratio;
    ortho.scale = new_scale;
}

/// マウスホイールで任意キャンバスをズームするジェネリックシステム。
fn zoom_canvas<C: Component + ZoomTowardsCursor>(
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    layout: Res<CanvasLayout>,
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

    if C::USE_RESULT_DEPTH_BLOCK {
        if let Some(pos) = window.cursor_position()
            && pos_on_result_depth_overlay(&layout, pos)
        {
            return;
        }
    }

    let Projection::Orthographic(ref mut ortho) = *proj else { return; };

    let old_scale = ortho.scale;
    let new_scale = (old_scale * (1.0 - total * ZOOM_SPEED)).clamp(ZOOM_MIN, ZOOM_MAX);
    let scale_ratio = new_scale / old_scale;

    if C::ZOOM_TOWARDS_CURSOR {
        if let Some(cursor_screen) = window.cursor_position() {
            if let Ok(cursor_world) = cam.viewport_to_world_2d(cam_tf, cursor_screen) {
                transform.translation.x +=
                    (cursor_world.x - transform.translation.x) * (1.0 - scale_ratio);
                transform.translation.y +=
                    (cursor_world.y - transform.translation.y) * (1.0 - scale_ratio);
            }
        }
    } else {
        transform.translation.x *= scale_ratio;
        transform.translation.y *= scale_ratio;
    }

    ortho.scale = new_scale;
}

/// ピンチ量をカメラに適用する（スクリーン上の点をズームの中心に固定する）。
fn apply_pinch_to_cam(
    cam: &Camera,
    cam_tf: &GlobalTransform,
    proj: &mut Projection,
    transform: &mut Transform,
    midpoint: Vec2,
    scale_factor: f32,
) {
    let Projection::Orthographic(ref mut ortho) = *proj else { return; };
    let old_scale = ortho.scale;
    let new_scale = (old_scale * scale_factor).clamp(ZOOM_MIN, ZOOM_MAX);
    let scale_ratio = new_scale / old_scale;

    if let Ok(mid_world) = cam.viewport_to_world_2d(cam_tf, midpoint) {
        transform.translation.x += (mid_world.x - transform.translation.x) * (1.0 - scale_ratio);
        transform.translation.y += (mid_world.y - transform.translation.y) * (1.0 - scale_ratio);
    }
    ortho.scale = new_scale;
}

/// ワールド原点をズームの中心に固定してスケールを適用する（`zoom_canvas` の非カーソル分岐と同じ）。
fn apply_pinch_at_world_origin(
    proj: &mut Projection,
    transform: &mut Transform,
    scale_factor: f32,
) {
    let Projection::Orthographic(ref mut ortho) = *proj else { return; };
    let old_scale = ortho.scale;
    let new_scale = (old_scale * scale_factor).clamp(ZOOM_MIN, ZOOM_MAX);
    let scale_ratio = new_scale / old_scale;
    transform.translation.x *= scale_ratio;
    transform.translation.y *= scale_ratio;
    ortho.scale = new_scale;
}

/// 2 本指ピンチで Edit / Placement / Result カメラをズームする。
fn handle_pinch_zoom(
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    layout: Res<CanvasLayout>,
    mut pinch: ResMut<PinchState>,
    placement: Res<PlacementState>,
    mut state: ResMut<FractalState>,
    mut edit_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Projection, &mut Transform),
        (With<EditCamera>, Without<PlacementCamera>, Without<ResultCamera>),
    >,
    mut placement_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Projection, &mut Transform),
        (With<PlacementCamera>, Without<EditCamera>, Without<ResultCamera>),
    >,
    mut result_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Projection, &mut Transform),
        (With<ResultCamera>, Without<EditCamera>, Without<PlacementCamera>),
    >,
) {
    let Ok(window) = windows.single() else { return; };
    let positions: Vec<Vec2> = touches.iter().map(|t| t.position()).collect();

    if positions.len() != 2 {
        pinch.active = false;
        pinch.prev_distance = 0.0;
        pinch.target = PinchTarget::None;
        return;
    }

    let distance = (positions[1] - positions[0]).length();
    let midpoint = (positions[0] + positions[1]) * 0.5;

    // ピンチ開始時に対象カメラを決定する
    if !pinch.active {
        let e_in = edit_cam_q.single().map(|(c, _, _, _)| pos_in_viewport(midpoint, window, c)).unwrap_or(false);
        let p_in = placement_cam_q.single().map(|(c, _, _, _)| pos_in_viewport(midpoint, window, c)).unwrap_or(false);
        let r_in = result_cam_q.single().map(|(c, _, _, _)| pos_in_viewport(midpoint, window, c)).unwrap_or(false);

        pinch.target = if e_in {
            PinchTarget::Edit
        } else if p_in {
            PinchTarget::Placement
        } else if r_in && !pos_on_result_depth_overlay(&layout, midpoint) {
            PinchTarget::Result
        } else {
            PinchTarget::None
        };
    }

    if pinch.active && pinch.prev_distance > 0.0 {
        let scale_factor = pinch.prev_distance / distance;
        match pinch.target {
            PinchTarget::Edit => {
                if let Ok((_, _, mut proj, mut transform)) = edit_cam_q.single_mut() {
                    apply_pinch_at_world_origin(&mut proj, &mut transform, scale_factor);
                }
            }
            PinchTarget::Placement => {
                if let Some(sel) = placement.selected.filter(|&i| i < state.replicas.len()) {
                    if let Some(replica) = state.replicas.get_mut(sel) {
                        // ortho と同じ prev/distance だとピンチの見た目と逆（離すと縮む）になる。指を広げて拡大に揃える。
                        let replica_factor = distance / pinch.prev_distance;
                        apply_replica_scale_factor(replica, replica_factor);
                    }
                } else if let Ok((_, _, mut proj, mut transform)) = placement_cam_q.single_mut() {
                    apply_pinch_at_world_origin(&mut proj, &mut transform, scale_factor);
                }
            }
            PinchTarget::Result => {
                if let Ok((cam, cam_tf, mut proj, mut transform)) = result_cam_q.single_mut() {
                    apply_pinch_to_cam(cam, cam_tf, &mut proj, &mut transform, midpoint, scale_factor);
                }
            }
            PinchTarget::None => {}
        }
    }

    pinch.active = true;
    pinch.prev_distance = distance;
    pinch.prev_midpoint = midpoint;
}

/// マウスドラッグ・タッチ 1 本指で Result カメラをパン（平行移動）する。
fn pan_result(
    mut pan_state: ResMut<PanState>,
    pinch: Res<PinchState>,
    dtap: Res<DoubleTapZoomActive>,
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    layout: Res<CanvasLayout>,
    mut result_cam_q: Query<(&Camera, &GlobalTransform, &mut Transform), With<ResultCamera>>,
) {
    let Ok(window) = windows.single() else { return; };
    let Ok((cam, cam_tf, mut transform)) = result_cam_q.single_mut() else { return; };

    // === タッチパン（1 本指）===

    // ピンチ中またはダブルタップズーム中はタッチパンをキャンセル
    if pinch.active || dtap.0 {
        pan_state.touch_id = None;
    }

    // 2 本指になったらキャンセル
    if pan_state.touch_id.is_some() && touches.iter().count() >= 2 {
        pan_state.touch_id = None;
    }

    // タッチ解放を追跡
    for touch in touches.iter_just_released() {
        if Some(touch.id()) == pan_state.touch_id {
            pan_state.touch_id = None;
        }
    }

    // 新しいタッチ開始（Result ビューポート内、ピンチ中でない場合）
    if pan_state.touch_id.is_none() && !pinch.active {
        for touch in touches.iter_just_pressed() {
            if pan_state.touch_id.is_some() {
                // 2 本目が来たらキャンセル
                pan_state.touch_id = None;
                break;
            }
            let pos = touch.position();
            if pos_in_viewport(pos, window, cam) && !pos_on_result_depth_overlay(&layout, pos) {
                pan_state.touch_id = Some(touch.id());
                pan_state.touch_last_pos = pos;
            }
        }
    }

    // タッチパン適用
    if let Some(id) = pan_state.touch_id {
        if let Some(touch) = touches.iter().find(|t| t.id() == id) {
            let curr_pos = touch.position();
            if let (Ok(prev_world), Ok(curr_world)) = (
                cam.viewport_to_world_2d(cam_tf, pan_state.touch_last_pos),
                cam.viewport_to_world_2d(cam_tf, curr_pos),
            ) {
                let delta = curr_world - prev_world;
                transform.translation.x -= delta.x;
                transform.translation.y -= delta.y;
            }
            pan_state.touch_last_pos = curr_pos;
        }
    }

    // === マウスパン ===

    let Some(cursor_screen) = window.cursor_position() else {
        pan_state.mouse_active = false;
        return;
    };

    if buttons.just_released(MouseButton::Left) {
        pan_state.mouse_active = false;
    }

    if buttons.just_pressed(MouseButton::Left)
        && cursor_in_viewport(window, cam)
        && !pos_on_result_depth_overlay(&layout, cursor_screen)
    {
        pan_state.mouse_active = true;
        pan_state.mouse_last_pos = cursor_screen;
    }

    if pan_state.mouse_active && buttons.pressed(MouseButton::Left) {
        if let (Ok(prev_world), Ok(curr_world)) = (
            cam.viewport_to_world_2d(cam_tf, pan_state.mouse_last_pos),
            cam.viewport_to_world_2d(cam_tf, cursor_screen),
        ) {
            let delta = curr_world - prev_world;
            transform.translation.x -= delta.x;
            transform.translation.y -= delta.y;
        }
        pan_state.mouse_last_pos = cursor_screen;
    }
}

/// ダブルタップ後に上下ドラッグでズームするジェスチャーを処理する。
/// - 上ドラッグ: ズームアウト（画面上は小さく見える）
/// - 下ドラッグ: ズームイン
/// Placement でレプリカ選択中も、カメラと同じ見た目になるようレプリカ倍率には `zoom_factor` の逆数を掛ける。
#[allow(clippy::too_many_arguments)]
fn handle_double_tap_zoom(
    time: Res<Time>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    layout: Res<CanvasLayout>,
    mut dtap_state: ResMut<DoubleTapZoomState>,
    mut dtap_active: ResMut<DoubleTapZoomActive>,
    pinch: Res<PinchState>,
    placement: Res<PlacementState>,
    mut state: ResMut<FractalState>,
    mut edit_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Projection, &mut Transform),
        (With<EditCamera>, Without<PlacementCamera>, Without<ResultCamera>),
    >,
    mut placement_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Projection, &mut Transform),
        (With<PlacementCamera>, Without<EditCamera>, Without<ResultCamera>),
    >,
    mut result_cam_q: Query<
        (&Camera, &GlobalTransform, &mut Projection, &mut Transform),
        (With<ResultCamera>, Without<EditCamera>, Without<PlacementCamera>),
    >,
) {
    let Ok(window) = windows.single() else { return; };
    let now = time.elapsed_secs_f64();

    // === アクティブなドラッグズームを処理 ===
    if let Some(id) = dtap_state.active_id {
        // ピンチが始まったらダブルタップズームをキャンセル
        if pinch.active || touches.iter().count() >= 2 {
            dtap_state.active_id = None;
            dtap_state.last_tap_time = 0.0;
            dtap_active.0 = false;
            return;
        }

        // タッチが離れたら終了
        if touches.iter_just_released().any(|t| t.id() == id) || touches.iter().find(|t| t.id() == id).is_none() {
            dtap_state.active_id = None;
            dtap_state.last_tap_time = 0.0; // 連続ダブルタップを防ぐ
            dtap_active.0 = false;
            return;
        }

        // Y 差分でズームを適用
        if let Some(touch) = touches.iter().find(|t| t.id() == id) {
            let curr_y = touch.position().y;
            let delta_y = curr_y - dtap_state.last_y;
            let zoom_factor = (1.0 - delta_y * DOUBLE_TAP_ZOOM_SPEED).max(0.01);

            match dtap_state.active_target {
                PinchTarget::Edit => {
                    if let Ok((_, _, mut proj, mut transform)) = edit_cam_q.single_mut() {
                        apply_pinch_at_world_origin(&mut proj, &mut transform, zoom_factor);
                    }
                }
                PinchTarget::Placement => {
                    if let Some(sel) = placement.selected.filter(|&i| i < state.replicas.len()) {
                        if let Some(replica) = state.replicas.get_mut(sel) {
                            // カメラは ortho.scale *= zoom_factor。レプリカは同じジェスチャで見た目を揃えるため逆数を掛ける。
                            let replica_factor = 1.0 / zoom_factor;
                            apply_replica_scale_factor(replica, replica_factor);
                        }
                    } else if let Ok((_, _, mut proj, mut transform)) = placement_cam_q.single_mut() {
                        apply_pinch_at_world_origin(&mut proj, &mut transform, zoom_factor);
                    }
                }
                PinchTarget::Result => {
                    if let Ok((cam, cam_tf, mut proj, mut transform)) = result_cam_q.single_mut() {
                        apply_pinch_to_cam(cam, cam_tf, &mut proj, &mut transform, touch.position(), zoom_factor);
                    }
                }
                PinchTarget::None => {}
            }

            dtap_state.last_y = curr_y;
        }
        return;
    }

    dtap_active.0 = false;

    // === ダブルタップを検出 ===
    for touch in touches.iter_just_pressed() {
        let pos = touch.position();
        let time_diff = now - dtap_state.last_tap_time;
        let dist = (pos - dtap_state.last_tap_pos).length();

        if time_diff < DOUBLE_TAP_TIME && dist < DOUBLE_TAP_DIST {
            // 対象ビューポートを特定（イミュータブル借用を即座にドロップ）
            let e_in = edit_cam_q.single().map(|(c, _, _, _)| pos_in_viewport(pos, window, c)).unwrap_or(false);
            let p_in = placement_cam_q.single().map(|(c, _, _, _)| pos_in_viewport(pos, window, c)).unwrap_or(false);
            let r_in = result_cam_q.single().map(|(c, _, _, _)| pos_in_viewport(pos, window, c)).unwrap_or(false);
            let target = if e_in {
                PinchTarget::Edit
            } else if p_in {
                PinchTarget::Placement
            } else if r_in && !pos_on_result_depth_overlay(&layout, pos) {
                PinchTarget::Result
            } else {
                PinchTarget::None
            };

            if target != PinchTarget::None {
                dtap_state.active_id = Some(touch.id());
                dtap_state.active_target = target;
                dtap_state.last_y = pos.y;
                dtap_state.last_tap_time = 0.0;
                dtap_active.0 = true;
            }
        } else {
            // 1 回目のタップとして記録
            dtap_state.last_tap_time = now;
            dtap_state.last_tap_pos = pos;
        }
    }
}

/// [`main::normalized_projection`] と同じ AutoMin ルールで、ビューポートあたりの論理投影サイズ（ワールド単位ベース）を返す。
fn ortho_automin_projection_size(viewport_w: f32, viewport_h: f32) -> (f32, f32) {
    let mw = RESULT_ORTHO_AUTOMIN;
    let mh = RESULT_ORTHO_AUTOMIN;
    if viewport_w * mh > mw * viewport_h {
        (viewport_w * mh / viewport_h, mh)
    } else {
        (mw, viewport_h * mw / viewport_w)
    }
}

/// params_panel が Result のビューポートを更新した直後に実行し、保留中なら図形に合わせてズームと中心を付ける。
pub fn fit_result_camera_if_requested(
    mut pending: ResMut<PendingResultCameraFit>,
    state: Res<FractalState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut result_cam: Query<(&Camera, &mut Projection, &mut Transform), With<ResultCamera>>,
) {
    if !pending.0 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((cam, mut proj, mut tf)) = result_cam.single_mut() else {
        return;
    };
    let Some(vp) = cam.viewport.as_ref() else {
        return;
    };
    let sf = window.scale_factor();
    let vw = vp.physical_size.x as f32 / sf;
    let vh = vp.physical_size.y as f32 / sf;
    if vw < 2.0 || vh < 2.0 {
        return;
    }

    let vh_fit = (vh - RESULT_FIT_DEPTH_OVERLAY_STRIP_PX).max(48.0);
    let (mut min_v, mut max_v) = match fractal_world_aabb(&state) {
        Some(b) => b,
        None => (Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)),
    };
    let span_w = (max_v.x - min_v.x).max(1e-6);
    let span_h = (max_v.y - min_v.y).max(1e-6);
    let pad = RESULT_FIT_PADDING_RATIO * span_w.max(span_h);
    min_v -= Vec2::splat(pad);
    max_v += Vec2::splat(pad);
    let bw = (max_v.x - min_v.x).max(1e-6);
    let bh = (max_v.y - min_v.y).max(1e-6);

    let (proj_w, proj_h) = ortho_automin_projection_size(vw, vh_fit);
    let scale_fit = (bw / proj_w).max(bh / proj_h).clamp(ZOOM_MIN, ZOOM_MAX);

    let cx = (min_v.x + max_v.x) * 0.5;
    let cy = (min_v.y + max_v.y) * 0.5;
    // Y 上向き：カメラ中心を図形より少し上に置くと、図形は画面上でやや下へ寄り、左上 Depth 帯と重なりにくい。
    let cy_biased = cy + RESULT_FIT_TOP_SCREEN_BIAS_FRAC * scale_fit * proj_h;

    let Projection::Orthographic(ref mut ortho) = *proj else {
        return;
    };
    ortho.scale = scale_fit;
    tf.translation.x = cx;
    tf.translation.y = cy_biased;

    pending.0 = false;
}
