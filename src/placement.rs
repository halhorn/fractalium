//! Placement キャンバス（左下パネル）の描画とレプリカ操作インタラクションを提供するモジュール。
//!
//! - 基図形をゴースト表示、各レプリカを対応色で表示
//! - クリックで選択（レプリカ座標系に沿った外枠表示）
//! - ドラッグで移動（枠内）・スケール（表示枠の四隅付近）・回転（枠外）
//! - Ctrl: グリッドドット表示 + スナップ（移動: 細かいグリッド, スケール: 0.05, 回転: 15°）
//! - 右クリックで削除
//! - Ctrl+C / Ctrl+V でレプリカのコピペ
//! - Escape で選択解除

use std::f32::consts::PI;

use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::PlacementCamera;
use crate::fractal::result_replica_color;
use crate::grid::{draw_fine_grid, snap_to_fine_grid};
use crate::placement_layer;
use crate::state::{
    CanvasLayout, DoubleTapZoomActive, FractalState, Line, PlacementDrag, PlacementState,
    REPLICA_SCALE_MAX, REPLICA_SCALE_MIN, Replica, UndoStack,
};

// === 定数 ===

const GHOST_ALPHA: f32 = 0.20;
const REPLICA_ALPHA: f32 = 0.85;
const GIZMO_LINE_WIDTH_PX: f32 = 3.25;
/// クリックヒットテスト用マージン（表示 AABB をこの分だけ広げてヒット判定）。
const CLICK_MARGIN: f32 = 0.03;
/// 基図形が空の場合のデフォルト AABB 半辺長。
const DEFAULT_AABB_HALF: f32 = 0.08;
/// 表示枠を図形より広げるマージン（線が被って見えなくなるのを防ぐ）。
const DISPLAY_MARGIN: f32 = 0.05;
const SELECTION_COLOR: Color = Color::srgba(1.0, 1.0, 0.4, 0.9);
const PIVOT_COLOR: Color = Color::srgba(1.0, 1.0, 0.4, 0.35);
const HANDLE_HALF: f32 = 0.022;
/// スケール操作に反応する範囲：外枠の各頂点からの距離がこれ以下ならドラッグでスケール（キャンバス正規化座標）。
/// コーナーハンドル表示とつかみやすさを揃える。
const CORNER_SCALE_RADIUS: f32 = HANDLE_HALF * 3.0;
const PIVOT_ARM: f32 = 0.06;
/// RotatePending → Rotate に昇格する最小移動距離²（これ未満は単発クリックと判定）。
const ROTATE_START_THRESHOLD_SQ: f32 = 0.015 * 0.015;
/// コピペ時の位置オフセット。
const PASTE_OFFSET: Vec2 = Vec2::new(0.07, -0.07);

// === ギズモグループ ===

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct PlacementGizmos;

// === プラグイン ===

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        let mut config = bevy::gizmos::config::GizmoConfig {
            render_layers: placement_layer(),
            ..Default::default()
        };
        config.line.width = GIZMO_LINE_WIDTH_PX;

        app.init_resource::<PlacementState>()
            .insert_gizmo_config(PlacementGizmos, config)
            .add_systems(
                Update,
                (
                    handle_placement_input,
                    update_placement_cursor,
                    draw_ghost_base,
                    draw_replica_shapes,
                    draw_selection_box,
                    draw_placement_ctrl_grid,
                )
                    .chain(),
            )
            .add_systems(EguiPrimaryContextPass, placement_overlay_ui);
    }
}

// === AABB ヘルパー ===

#[derive(Clone, Copy)]
struct Aabb {
    min: Vec2,
    max: Vec2,
}

impl Aabb {
    fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    fn contains(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    fn expanded(self, margin: f32) -> Self {
        Self {
            min: self.min - Vec2::splat(margin),
            max: self.max + Vec2::splat(margin),
        }
    }
}

/// 基図形空間でのジオメトリ AABB（レプリカ適用前）。
fn base_shape_aabb(lines: &[Line]) -> Aabb {
    if lines.is_empty() {
        let h = DEFAULT_AABB_HALF;
        return Aabb {
            min: Vec2::splat(-h),
            max: Vec2::splat(h),
        };
    }
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for line in lines {
        for p in [line.a, line.b] {
            min = min.min(p);
            max = max.max(p);
        }
    }
    let center = (min + max) * 0.5;
    let half = ((max - min) * 0.5).max(Vec2::splat(DEFAULT_AABB_HALF));
    Aabb {
        min: center - half,
        max: center + half,
    }
}

/// 基図形空間での表示用 AABB（マージン付き）。レプリカと同じ向きの外枠のローカル定義。
fn base_display_aabb(lines: &[Line]) -> Aabb {
    base_shape_aabb(lines).expanded(DISPLAY_MARGIN)
}

/// 表示外枠の中心をワールド座標で返す（スケール・回転のピボット）。
fn display_frame_center_world(replica: &Replica, lines: &[Line]) -> Vec2 {
    replica.apply(base_display_aabb(lines).center())
}

/// ワールド点が表示外枠（レプリカ座標系に固まった矩形）内か。`margin` は基図形空間で広げる。
fn display_frame_contains_world(
    replica: &Replica,
    lines: &[Line],
    world: Vec2,
    margin: f32,
) -> bool {
    let local = replica.inverse_apply(world);
    base_display_aabb(lines).expanded(margin).contains(local)
}

/// 表示外枠の四隅いずれかから `CORNER_SCALE_RADIUS` 以内ならスケール操作対象。
fn display_frame_near_corner_world(replica: &Replica, lines: &[Line], world: Vec2) -> bool {
    let r_sq = CORNER_SCALE_RADIUS * CORNER_SCALE_RADIUS;
    oriented_display_corners(replica, lines)
        .into_iter()
        .any(|c| (c - world).length_squared() <= r_sq)
}

fn oriented_display_corners(replica: &Replica, lines: &[Line]) -> [Vec2; 4] {
    let a = base_display_aabb(lines);
    let mn = a.min;
    let mx = a.max;
    [
        replica.apply(Vec2::new(mn.x, mn.y)),
        replica.apply(Vec2::new(mx.x, mn.y)),
        replica.apply(Vec2::new(mx.x, mx.y)),
        replica.apply(Vec2::new(mn.x, mx.y)),
    ]
}

// === カーソル変換 ===

fn world_pos_in_placement(
    screen: Vec2,
    window: &Window,
    cam: &Camera,
    cam_tf: &GlobalTransform,
) -> Option<Vec2> {
    if let Some(ref vp) = cam.viewport {
        let scale = window.scale_factor();
        let vp_min = vp.physical_position.as_vec2() / scale;
        let vp_size = vp.physical_size.as_vec2() / scale;
        if screen.x < vp_min.x
            || screen.y < vp_min.y
            || screen.x > vp_min.x + vp_size.x
            || screen.y > vp_min.y + vp_size.y
        {
            return None;
        }
    }
    cam.viewport_to_world_2d(cam_tf, screen).ok()
}

fn cursor_in_placement(window: &Window, cam: &Camera, cam_tf: &GlobalTransform) -> Option<Vec2> {
    world_pos_in_placement(window.cursor_position()?, window, cam, cam_tf)
}

/// ワールド座標 → egui 画面論理座標（Placement カメラ使用）。
fn world_to_egui_pos(
    world: Vec2,
    cam: &Camera,
    cam_tf: &GlobalTransform,
    scale: f32,
) -> Option<egui::Pos2> {
    let vp_log = cam
        .world_to_viewport(cam_tf, Vec3::new(world.x, world.y, 0.0))
        .ok()?;
    let vp_origin_log = cam
        .viewport
        .as_ref()
        .map(|v| v.physical_position.as_vec2() / scale)
        .unwrap_or(Vec2::ZERO);
    let screen_log = vp_log + vp_origin_log;
    Some(egui::pos2(screen_log.x, screen_log.y))
}

// === 入力ハンドラ ===

#[allow(clippy::too_many_arguments)]
fn handle_placement_input(
    mut state: ResMut<FractalState>,
    mut placement: ResMut<PlacementState>,
    mut undo_stack: ResMut<UndoStack>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    dtap_active: Res<DoubleTapZoomActive>,
    mut drag_touch_id: Local<Option<u64>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    placement_cam: Query<(&Camera, &GlobalTransform), With<PlacementCamera>>,
    mut contexts: EguiContexts,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((cam, cam_tf)) = placement_cam.single() else {
        return;
    };
    let ctrl = keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || state.snap_grid;

    // === タッチ入力管理 ===
    let touch_count = touches.iter().count();

    // ダブルタップズーム中はタッチ操作をキャンセル・スキップ
    if dtap_active.0 && drag_touch_id.is_some() {
        *drag_touch_id = None;
        if !matches!(placement.drag, PlacementDrag::Idle) {
            placement.drag = PlacementDrag::Idle;
        }
    }

    if touch_count >= 2 || dtap_active.0 {
        if drag_touch_id.is_some() {
            *drag_touch_id = None;
            if !matches!(placement.drag, PlacementDrag::Idle) {
                placement.drag = PlacementDrag::Idle;
            }
        }
    } else if drag_touch_id.is_none() {
        if let Some(t) = touches.iter_just_pressed().next() {
            *drag_touch_id = Some(t.id());
        }
    }

    let (touch_just_pressed, touch_pressed, touch_just_released, touch_screen_pos): (
        bool,
        bool,
        bool,
        Option<Vec2>,
    ) = if let Some(drag_id) = *drag_touch_id {
        let active = touches.iter().find(|t| t.id() == drag_id);
        let released = touches.iter_just_released().find(|t| t.id() == drag_id);
        let just_pressed = touches.just_pressed(drag_id);
        let just_released = released.is_some();
        let screen_pos = active
            .map(|t| t.position())
            .or_else(|| released.map(|t| t.position()));
        if just_released {
            *drag_touch_id = None;
        }
        (just_pressed, active.is_some(), just_released, screen_pos)
    } else {
        (false, false, false, None)
    };

    let use_touch = touch_pressed || touch_just_pressed || touch_just_released;
    let pressing = if use_touch {
        touch_pressed
    } else {
        buttons.pressed(MouseButton::Left)
    };
    let just_pressing = if use_touch {
        touch_just_pressed
    } else {
        buttons.just_pressed(MouseButton::Left)
    };
    let just_releasing = if use_touch {
        touch_just_released
    } else {
        buttons.just_released(MouseButton::Left)
    };

    let cursor: Option<Vec2> = touch_screen_pos
        .or_else(|| window.cursor_position())
        .and_then(|pos| world_pos_in_placement(pos, window, cam, cam_tf));

    // Escape: 選択解除
    if keys.just_pressed(KeyCode::Escape) {
        placement.selected = None;
        placement.drag = PlacementDrag::Idle;
        return;
    }

    // Backspace / Delete: 選択中レプリカを削除
    if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::Delete) {
        if let Some(sel) = placement.selected {
            if sel < state.replicas.len() {
                undo_stack.push(state.clone());
                state.replicas.remove(sel);
                placement.selected = None;
                placement.drag = PlacementDrag::Idle;
            }
        }
        return;
    }

    let egui_ctx = contexts.ctx_mut();
    let egui_wants_pointer = egui_ctx
        .as_ref()
        .map(|ctx| ctx.is_using_pointer())
        .unwrap_or(false);
    let egui_wants_keyboard = egui_ctx
        .map(|ctx| ctx.wants_keyboard_input())
        .unwrap_or(false);

    // Ctrl+C / Ctrl+V（egui がキーボードを必要としていないときのみ）
    if ctrl && !egui_wants_keyboard {
        if keys.just_pressed(KeyCode::KeyC) {
            if let Some(sel) = placement.selected {
                placement.clipboard = state.replicas.get(sel).copied();
            }
        }
        if keys.just_pressed(KeyCode::KeyV) {
            if let Some(ref mut cb) = placement.clipboard {
                cb.translation += PASTE_OFFSET;
                let new_replica = *cb;
                undo_stack.push(state.clone());
                let new_idx = state.replicas.len();
                state.replicas.push(new_replica);
                placement.selected = Some(new_idx);
            }
        }
    }

    // ドラッグ継続・終了は egui_wants_pointer に関わらず処理する。
    if pressing {
        if let PlacementDrag::RotatePending {
            pivot,
            start_cursor,
            start_angle,
            start_rotation,
        } = placement.drag
        {
            if let Some(pos) = cursor {
                if (pos - start_cursor).length_squared() > ROTATE_START_THRESHOLD_SQ {
                    undo_stack.push(state.clone());
                    placement.drag = PlacementDrag::Rotate {
                        pivot,
                        start_angle,
                        start_rotation,
                    };
                }
            }
        }
        if let Some(pos) = cursor {
            apply_drag(pos, ctrl, &mut state, &mut placement);
        }
    }
    if just_releasing && !matches!(placement.drag, PlacementDrag::Idle) {
        if matches!(placement.drag, PlacementDrag::RotatePending { .. }) {
            placement.selected = None;
        }
        placement.drag = PlacementDrag::Idle;
    }

    if egui_wants_pointer {
        return;
    }

    // 左クリック押下
    if just_pressing {
        if let Some(pos) = cursor {
            start_drag(pos, &mut state, &mut placement, &mut undo_stack);
        }
    }

    // 右クリック押下 → 選択中レプリカを削除
    if buttons.just_pressed(MouseButton::Right) {
        if let Some(pos) = cursor {
            // クリック位置のレプリカを探す
            let hit = state
                .replicas
                .iter()
                .enumerate()
                .rev()
                .find(|(_, r)| {
                    display_frame_contains_world(r, &state.base_shape.lines, pos, CLICK_MARGIN)
                })
                .map(|(i, _)| i);

            if let Some(i) = hit {
                undo_stack.push(state.clone());
                state.replicas.remove(i);
                placement.selected = placement.selected.and_then(|sel| match sel.cmp(&i) {
                    std::cmp::Ordering::Equal => None,
                    std::cmp::Ordering::Greater => Some(sel - 1),
                    std::cmp::Ordering::Less => Some(sel),
                });
                placement.drag = PlacementDrag::Idle;
            }
        }
    }
}

/// マウス押下時のドラッグ開始判定。
/// スケール判定は表示外枠の四隅から一定距離以内に限定する。
fn start_drag(
    cursor_pos: Vec2,
    state: &mut FractalState,
    placement: &mut PlacementState,
    undo_stack: &mut UndoStack,
) {
    // 選択中レプリカを最優先でヒットテスト（枠線ドラッグを他のオブジェクトに奪われないため）
    let selected_hit = placement.selected.and_then(|sel| {
        let r = state.replicas.get(sel)?;
        display_frame_contains_world(r, &state.base_shape.lines, cursor_pos, CLICK_MARGIN)
            .then_some(sel)
    });

    // 選択中がヒットしない場合のみ他のレプリカを検索（描画順の逆＝手前優先）
    let clicked = selected_hit.or_else(|| {
        state
            .replicas
            .iter()
            .enumerate()
            .rev()
            .find(|(i, r)| {
                Some(*i) != placement.selected
                    && display_frame_contains_world(
                        r,
                        &state.base_shape.lines,
                        cursor_pos,
                        CLICK_MARGIN,
                    )
            })
            .map(|(i, _)| i)
    });

    if let Some(i) = clicked {
        placement.selected = Some(i);
        let r = &state.replicas[i];
        let lines = &state.base_shape.lines;
        let drag = if display_frame_near_corner_world(r, lines, cursor_pos) {
            // 表示枠の四隅付近 → スケール
            let pivot = display_frame_center_world(r, lines);
            PlacementDrag::Scale {
                pivot,
                start_cursor_dist: (cursor_pos - pivot).length().max(1e-4),
                start_scale: state.replicas[i].scale,
            }
        } else {
            // 枠内 → 移動
            PlacementDrag::Move {
                start_cursor: cursor_pos,
                start_translation: state.replicas[i].translation,
            }
        };
        undo_stack.push(state.clone());
        placement.drag = drag;
    } else if let Some(sel) = placement.selected {
        // 全レプリカ外 → 回転待機（単発クリックなら選択解除、ドラッグなら回転）
        if sel < state.replicas.len() {
            let pivot = display_frame_center_world(&state.replicas[sel], &state.base_shape.lines);
            let d = cursor_pos - pivot;
            placement.drag = PlacementDrag::RotatePending {
                pivot,
                start_cursor: cursor_pos,
                start_angle: d.y.atan2(d.x),
                start_rotation: state.replicas[sel].rotation,
            };
        }
    }
}

/// ドラッグ中にレプリカのパラメータを更新する。
fn apply_drag(
    cursor_pos: Vec2,
    ctrl: bool,
    state: &mut FractalState,
    placement: &mut PlacementState,
) {
    let Some(sel) = placement.selected else {
        return;
    };
    let Some(replica) = state.replicas.get_mut(sel) else {
        return;
    };
    match placement.drag {
        PlacementDrag::Idle => {}
        PlacementDrag::Move {
            start_cursor,
            start_translation,
        } => {
            let raw = start_translation + (cursor_pos - start_cursor);
            replica.translation = if ctrl { snap_to_fine_grid(raw) } else { raw };
        }
        PlacementDrag::Scale {
            pivot,
            start_cursor_dist,
            start_scale,
        } => {
            let current_dist = (cursor_pos - pivot).length();
            let raw_scale = start_scale * (current_dist / start_cursor_dist);
            replica.scale = if ctrl {
                (raw_scale / 0.05).round() * 0.05
            } else {
                raw_scale
            }
            .clamp(REPLICA_SCALE_MIN, REPLICA_SCALE_MAX);
        }
        PlacementDrag::Rotate {
            pivot,
            start_angle,
            start_rotation,
        } => {
            let d = cursor_pos - pivot;
            let current_angle = d.y.atan2(d.x);
            let raw = start_rotation + angle_diff(current_angle, start_angle);
            replica.rotation = if ctrl {
                let step = PI / 12.0; // 15°
                (raw / step).round() * step
            } else {
                raw
            };
        }
        PlacementDrag::RotatePending { .. } => {}
    }
}

fn angle_diff(a: f32, b: f32) -> f32 {
    let d = a - b;
    (d + PI).rem_euclid(2.0 * PI) - PI
}

// === カーソルアイコン更新 ===

/// 表示枠の四隅付近にポインタが当たったときにカーソルアイコンを変える。
fn update_placement_cursor(
    mut commands: Commands,
    state: Res<FractalState>,
    placement: Res<PlacementState>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    placement_cam: Query<(&Camera, &GlobalTransform), With<PlacementCamera>>,
) {
    let Ok((window_entity, window)) = windows.single() else {
        return;
    };
    let Ok((cam, cam_tf)) = placement_cam.single() else {
        return;
    };
    let cursor = cursor_in_placement(window, cam, cam_tf);

    let on_corner = placement
        .selected
        .and_then(|sel| state.replicas.get(sel))
        .is_some_and(|r| {
            cursor
                .is_some_and(|pos| display_frame_near_corner_world(r, &state.base_shape.lines, pos))
        });

    if on_corner {
        commands
            .entity(window_entity)
            .insert(CursorIcon::from(SystemCursorIcon::NwseResize));
    } else {
        commands.entity(window_entity).remove::<CursorIcon>();
    }
}

// === Ctrl グリッド描画 ===

/// Ctrl 押下中に細かいグリッドドットを表示する。
fn draw_placement_ctrl_grid(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<FractalState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    placement_cam: Query<(&Camera, &GlobalTransform), With<PlacementCamera>>,
    mut gizmos: Gizmos<PlacementGizmos>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || state.snap_grid;
    if !ctrl {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((cam, cam_tf)) = placement_cam.single() else {
        return;
    };
    let cursor = cursor_in_placement(window, cam, cam_tf);
    draw_fine_grid(&mut gizmos, cursor.map(snap_to_fine_grid));
}

// === 描画システム ===

fn draw_ghost_base(state: Res<FractalState>, mut gizmos: Gizmos<PlacementGizmos>) {
    let color = Color::srgba(0.9, 0.9, 1.0, GHOST_ALPHA);
    for line in &state.base_shape.lines {
        gizmos.line_2d(line.a, line.b, color);
    }
}

fn draw_replica_shapes(state: Res<FractalState>, mut gizmos: Gizmos<PlacementGizmos>) {
    let total = state.replicas.len();
    for (i, replica) in state.replicas.iter().enumerate() {
        let lin = result_replica_color(i, total);
        let color = Color::from(LinearRgba::new(lin.red, lin.green, lin.blue, REPLICA_ALPHA));
        for line in &state.base_shape.lines {
            gizmos.line_2d(replica.apply(line.a), replica.apply(line.b), color);
        }
    }
}

fn draw_selection_box(
    state: Res<FractalState>,
    placement: Res<PlacementState>,
    mut gizmos: Gizmos<PlacementGizmos>,
) {
    let Some(sel) = placement.selected else {
        return;
    };
    let Some(replica) = state.replicas.get(sel) else {
        return;
    };
    let corners = oriented_display_corners(replica, &state.base_shape.lines);
    for k in 0..4 {
        gizmos.line_2d(corners[k], corners[(k + 1) % 4], SELECTION_COLOR);
    }

    // コーナーハンドル
    for corner in corners {
        let h = HANDLE_HALF;
        draw_rect(
            &mut gizmos,
            corner - Vec2::splat(h),
            corner + Vec2::splat(h),
            SELECTION_COLOR,
        );
    }

    // + マーク: レプリカの座標原点（translation）に表示
    let pivot = replica.translation;
    gizmos.line_2d(
        pivot - Vec2::X * PIVOT_ARM,
        pivot + Vec2::X * PIVOT_ARM,
        PIVOT_COLOR,
    );
    gizmos.line_2d(
        pivot - Vec2::Y * PIVOT_ARM,
        pivot + Vec2::Y * PIVOT_ARM,
        PIVOT_COLOR,
    );
}

fn draw_rect(gizmos: &mut Gizmos<PlacementGizmos>, min: Vec2, max: Vec2, color: Color) {
    gizmos.line_2d(Vec2::new(min.x, min.y), Vec2::new(max.x, min.y), color);
    gizmos.line_2d(Vec2::new(max.x, min.y), Vec2::new(max.x, max.y), color);
    gizmos.line_2d(Vec2::new(max.x, max.y), Vec2::new(min.x, max.y), color);
    gizmos.line_2d(Vec2::new(min.x, max.y), Vec2::new(min.x, min.y), color);
}

// === egui オーバーレイ（削除ボタン） ===

/// 選択中レプリカの表示枠の右上に × 削除ボタンを表示する。
#[allow(clippy::too_many_arguments)]
fn placement_overlay_ui(
    mut contexts: EguiContexts,
    mut state: ResMut<FractalState>,
    mut placement: ResMut<PlacementState>,
    mut undo_stack: ResMut<UndoStack>,
    windows: Query<&Window, With<PrimaryWindow>>,
    placement_cam: Query<(&Camera, &GlobalTransform), With<PlacementCamera>>,
    layout: Res<CanvasLayout>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let Some(sel) = placement.selected else {
        return Ok(());
    };
    let Some(replica) = state.replicas.get(sel) else {
        return Ok(());
    };
    let Ok(window) = windows.single() else {
        return Ok(());
    };
    let Ok((cam, cam_tf)) = placement_cam.single() else {
        return Ok(());
    };

    let scale = window.scale_factor();
    // 基図形ローカル (max x, max y) をワールドへ（レプリカと同じ向きの外枠の「右上」）
    let corners = oriented_display_corners(replica, &state.base_shape.lines);
    let Some(btn_pos) = world_to_egui_pos(corners[2], cam, cam_tf, scale) else {
        return Ok(());
    };

    // Placement パネル内に収まっているときのみ表示
    if btn_pos.x < layout.placement_min_x
        || btn_pos.x > layout.placement_max_x
        || btn_pos.y < layout.placement_min_y
        || btn_pos.y > layout.placement_max_y
    {
        return Ok(());
    }

    let mut delete = false;
    egui::Area::new("placement_delete_btn".into())
        .fixed_pos(egui::pos2(btn_pos.x, btn_pos.y - 20.0))
        .show(ctx, |ui| {
            if ui.button("✕").clicked() {
                delete = true;
            }
        });

    if delete {
        undo_stack.push(state.clone());
        state.replicas.remove(sel);
        placement.selected = None;
        placement.drag = PlacementDrag::Idle;
    }

    Ok(())
}
