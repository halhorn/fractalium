//! Edit キャンバス上での編集機能を提供するモジュール。
//!
//! 役割:
//! - マウスドラッグによる線分入力（Ctrl/Shift スナップ対応）
//! - 既存線のクリック選択・平行移動・端点個別移動
//! - Delete/Backspace で選択線を削除、Escape で選択解除
//! - Cmd+Z による Undo
//! - 確定済み線分・選択ハイライト・端点◯・ドラッグ中プレビューの描画

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::EditCamera;
use crate::edit_layer;
use crate::grid::{draw_grid, snap_to_grid};
use crate::state::{FractalState, Line, UndoStack};

const MIN_LINE_LEN: f32 = 0.01;
const CONFIRMED_COLOR: Color = Color::srgb(0.9, 0.9, 1.0);
const SELECTED_COLOR: Color = Color::srgb(1.0, 1.0, 0.45);
const PREVIEW_COLOR: Color = Color::srgb(1.0, 0.8, 0.4);
const SNAP_PREVIEW_COLOR: Color = Color::srgb(0.4, 1.0, 0.6);
const GRID_PREVIEW_COLOR: Color = Color::srgb(0.4, 0.8, 1.0);
const ENDPOINT_COLOR: Color = Color::srgba(1.0, 1.0, 0.45, 0.9);
/// 端点◯の表示半径。
const ENDPOINT_RADIUS: f32 = 0.035;
/// 端点のクリック判定半径（表示半径より広め）。
const ENDPOINT_HIT_RADIUS: f32 = 0.07;
/// 線本体のクリック判定距離。
const LINE_HIT_DISTANCE: f32 = 0.05;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct EditGizmos;

/// 線の端点 A / B を区別する。
#[derive(Clone, Copy)]
pub enum LineEnd {
    A,
    B,
}

/// Edit キャンバスの操作状態。
#[derive(Resource, Default)]
pub enum DrawState {
    /// 何も選択なし。ドラッグで新規線描画可能。
    #[default]
    Idle,
    /// 線を選択中（インデックス）。
    Selected(usize),
    /// 新しい線を描画中。
    DrawingLine {
        start: Vec2,
        /// リリースフレームで cursor が None のときのフォールバック。
        last_cursor: Vec2,
    },
    /// 選択中の線全体を平行移動中。
    MovingLine {
        idx: usize,
        cursor_start: Vec2,
        line_a_start: Vec2,
        line_b_start: Vec2,
    },
    /// 選択中の線の片端を移動中。
    MovingEndpoint {
        idx: usize,
        end: LineEnd,
        /// 動かさない側の端点（スナップ基準点として使用）。
        fixed: Vec2,
        last_cursor: Vec2,
    },
}

pub struct EditPlugin;

impl Plugin for EditPlugin {
    fn build(&self, app: &mut App) {
        let mut config = bevy::gizmos::config::GizmoConfig {
            render_layers: edit_layer(),
            ..Default::default()
        };
        config.line.width = 2.0;

        app.init_resource::<DrawState>()
            .init_resource::<UndoStack>()
            .insert_gizmo_config(EditGizmos, config)
            .add_systems(
                Update,
                (handle_undo, handle_drag_input, draw_canvas).chain(),
            );
    }
}

struct Modifiers {
    ctrl: bool,
    shift: bool,
}

impl Modifiers {
    fn read(keys: &ButtonInput<KeyCode>) -> Self {
        Self {
            ctrl: keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight),
            shift: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
        }
    }
}

/// 修飾キーに応じてドラッグ終点をスナップし、対応するプレビュー色とともに返す。
fn snap_endpoint(start: Vec2, raw_end: Vec2, modifiers: &Modifiers) -> (Vec2, Color) {
    if modifiers.ctrl {
        (snap_to_grid(raw_end), GRID_PREVIEW_COLOR)
    } else if modifiers.shift {
        (snap_to_45(start, raw_end), SNAP_PREVIEW_COLOR)
    } else {
        (raw_end, PREVIEW_COLOR)
    }
}

fn snap_to_45(start: Vec2, end: Vec2) -> Vec2 {
    let delta = end - start;
    if delta.length_squared() < f32::EPSILON {
        return end;
    }
    let angle = delta.y.atan2(delta.x);
    let snapped = (angle / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
    start + Vec2::new(snapped.cos(), snapped.sin()) * delta.length()
}

fn cursor_in_edit(window: &Window, cam: &Camera, cam_tf: &GlobalTransform) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    if let Some(ref vp) = cam.viewport {
        let scale = window.scale_factor();
        let vp_min = vp.physical_position.as_vec2() / scale;
        let vp_size = vp.physical_size.as_vec2() / scale;
        if cursor.x < vp_min.x
            || cursor.y < vp_min.y
            || cursor.x > vp_min.x + vp_size.x
            || cursor.y > vp_min.y + vp_size.y
        {
            return None;
        }
    }
    cam.viewport_to_world_2d(cam_tf, cursor).ok()
}

/// 点と線分の最短距離を返す。
fn point_to_segment_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < f32::EPSILON {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    (p - (a + t * ab)).length()
}

/// cursor に最も近い（かつ LINE_HIT_DISTANCE 以内の）線のインデックスを返す。
/// 描画順の逆（後から描かれた線を優先）で探す。
fn find_line_at(cursor: Vec2, lines: &[Line]) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_, l)| point_to_segment_dist(cursor, l.a, l.b) < LINE_HIT_DISTANCE)
        .map(|(i, _)| i)
}

fn handle_undo(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<FractalState>,
    mut undo_stack: ResMut<UndoStack>,
    mut draw_state: ResMut<DrawState>,
) {
    let cmd = keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight);
    if cmd
        && keys.just_pressed(KeyCode::KeyZ)
        && let Some(prev) = undo_stack.pop()
    {
        *state = prev;
        *draw_state = DrawState::Idle;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_drag_input(
    mut draw_state: ResMut<DrawState>,
    mut state: ResMut<FractalState>,
    mut undo_stack: ResMut<UndoStack>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    edit_cam: Query<(&Camera, &GlobalTransform), With<EditCamera>>,
    mut contexts: EguiContexts,
) {
    let Ok(window) = windows.single() else { return; };
    let Ok((cam, cam_tf)) = edit_cam.single() else { return; };
    let cursor = cursor_in_edit(window, cam, cam_tf);
    let modifiers = Modifiers::read(&keys);

    let egui_ctx = contexts.ctx_mut();
    let egui_wants_pointer = egui_ctx.as_ref().map(|ctx| ctx.is_using_pointer()).unwrap_or(false);
    let egui_wants_keyboard = egui_ctx.map(|ctx| ctx.wants_keyboard_input()).unwrap_or(false);

    // 選択インデックスが範囲外になっていたらリセット（Clear などによる削除対応）
    let n = state.base_shape.lines.len();
    let out_of_bounds = match *draw_state {
        DrawState::Selected(i)
        | DrawState::MovingLine { idx: i, .. }
        | DrawState::MovingEndpoint { idx: i, .. } => i >= n,
        _ => false,
    };
    if out_of_bounds {
        *draw_state = DrawState::Idle;
        return;
    }

    // Escape: 選択解除
    if keys.just_pressed(KeyCode::Escape) {
        *draw_state = DrawState::Idle;
        return;
    }

    // Delete / Backspace: 選択中の線を削除
    if !egui_wants_keyboard
        && (keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::Delete))
        && let DrawState::Selected(idx) = *draw_state
    {
        undo_stack.push(state.clone());
        state.base_shape.lines.remove(idx);
        *draw_state = DrawState::Idle;
        return;
    }

    // pressed: ドラッグ中の位置更新
    if buttons.pressed(MouseButton::Left) {
        match *draw_state {
            DrawState::DrawingLine { ref mut last_cursor, .. } => {
                if let Some(pos) = cursor {
                    *last_cursor = pos;
                }
            }
            DrawState::MovingLine { idx, cursor_start, line_a_start, line_b_start } => {
                if let Some(pos) = cursor {
                    let delta = pos - cursor_start;
                    if let Some(line) = state.base_shape.lines.get_mut(idx) {
                        line.a = line_a_start + delta;
                        line.b = line_b_start + delta;
                    }
                }
            }
            DrawState::MovingEndpoint { idx, end, fixed, ref mut last_cursor } => {
                if let Some(pos) = cursor {
                    *last_cursor = pos;
                    let (snapped, _) = snap_endpoint(fixed, pos, &modifiers);
                    if let Some(line) = state.base_shape.lines.get_mut(idx) {
                        match end {
                            LineEnd::A => line.a = snapped,
                            LineEnd::B => line.b = snapped,
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // just_released: ドラッグ確定 / 状態遷移
    if buttons.just_released(MouseButton::Left) {
        match *draw_state {
            DrawState::DrawingLine { start, last_cursor } => {
                let raw_end = cursor.unwrap_or(last_cursor);
                let (end, _) = snap_endpoint(start, raw_end, &modifiers);
                if (end - start).length() >= MIN_LINE_LEN {
                    undo_stack.push(state.clone());
                    state.base_shape.lines.push(Line { a: start, b: end });
                }
                *draw_state = DrawState::Idle;
            }
            DrawState::MovingLine { idx, .. } => {
                *draw_state = DrawState::Selected(idx);
            }
            DrawState::MovingEndpoint { idx, .. } => {
                *draw_state = DrawState::Selected(idx);
            }
            _ => {}
        }
    }

    // just_pressed: 新しい操作を開始する（優先順位順）
    if buttons.just_pressed(MouseButton::Left) && !egui_wants_pointer {
        if let Some(cursor_pos) = cursor {
            // 優先度 1: 選択中の線の端点◯をヒット → 端点移動開始
            let endpoint_hit = if let DrawState::Selected(sel) = *draw_state {
                state.base_shape.lines.get(sel).and_then(|line| {
                    if (cursor_pos - line.a).length() < ENDPOINT_HIT_RADIUS {
                        Some((sel, LineEnd::A, line.b))
                    } else if (cursor_pos - line.b).length() < ENDPOINT_HIT_RADIUS {
                        Some((sel, LineEnd::B, line.a))
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            if let Some((idx, end, fixed)) = endpoint_hit {
                undo_stack.push(state.clone());
                *draw_state = DrawState::MovingEndpoint { idx, end, fixed, last_cursor: cursor_pos };
            } else if let Some(hit_idx) = find_line_at(cursor_pos, &state.base_shape.lines) {
                // 優先度 2: 線本体をクリック → 選択 + 移動開始
                let line = state.base_shape.lines[hit_idx];
                undo_stack.push(state.clone());
                *draw_state = DrawState::MovingLine {
                    idx: hit_idx,
                    cursor_start: cursor_pos,
                    line_a_start: line.a,
                    line_b_start: line.b,
                };
            } else {
                // 優先度 3: 空白をクリック → 選択解除 + 新規線描画開始
                let start = if modifiers.ctrl { snap_to_grid(cursor_pos) } else { cursor_pos };
                *draw_state = DrawState::DrawingLine { start, last_cursor: start };
            }
        }
    }
}

fn draw_canvas(
    state: Res<FractalState>,
    draw_state: Res<DrawState>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    edit_cam: Query<(&Camera, &GlobalTransform), With<EditCamera>>,
    mut gizmos: Gizmos<EditGizmos>,
) {
    draw_confirmed_lines(&state, &draw_state, &mut gizmos);

    let modifiers = Modifiers::read(&keys);
    let cursor = windows
        .single()
        .ok()
        .zip(edit_cam.single().ok())
        .and_then(|(w, (cam, cam_tf))| cursor_in_edit(w, cam, cam_tf));

    if modifiers.ctrl {
        draw_grid(&mut gizmos, cursor.map(snap_to_grid));
    }

    // 新規線のドラッグプレビュー
    if let DrawState::DrawingLine { start, .. } = *draw_state
        && let Some(raw_end) = cursor
    {
        let (end, color) = snap_endpoint(start, raw_end, &modifiers);
        gizmos.line_2d(start, end, color);
    }

    // 選択中の線の端点◯
    let selected_idx = match *draw_state {
        DrawState::Selected(i)
        | DrawState::MovingLine { idx: i, .. }
        | DrawState::MovingEndpoint { idx: i, .. } => Some(i),
        _ => None,
    };
    if let Some(idx) = selected_idx
        && let Some(line) = state.base_shape.lines.get(idx)
    {
        draw_endpoint_circle(&mut gizmos, line.a);
        draw_endpoint_circle(&mut gizmos, line.b);
    }
}

fn draw_confirmed_lines(state: &FractalState, draw_state: &DrawState, gizmos: &mut Gizmos<EditGizmos>) {
    let selected_idx = match *draw_state {
        DrawState::Selected(i)
        | DrawState::MovingLine { idx: i, .. }
        | DrawState::MovingEndpoint { idx: i, .. } => Some(i),
        _ => None,
    };
    for (i, line) in state.base_shape.lines.iter().enumerate() {
        let color = if Some(i) == selected_idx { SELECTED_COLOR } else { CONFIRMED_COLOR };
        gizmos.line_2d(line.a, line.b, color);
    }
}

/// 端点を示す小円を線分で近似して描画する。
fn draw_endpoint_circle(gizmos: &mut Gizmos<EditGizmos>, center: Vec2) {
    const SEGMENTS: usize = 16;
    let step = std::f32::consts::TAU / SEGMENTS as f32;
    let mut prev = center + Vec2::new(ENDPOINT_RADIUS, 0.0);
    for i in 1..=SEGMENTS {
        let angle = i as f32 * step;
        let next = center + Vec2::new(angle.cos(), angle.sin()) * ENDPOINT_RADIUS;
        gizmos.line_2d(prev, next, ENDPOINT_COLOR);
        prev = next;
    }
}
