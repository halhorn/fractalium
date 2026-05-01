//! Edit キャンバス上での編集機能を提供するモジュール。
//!
//! 役割:
//! - マウスドラッグによる線分入力（Ctrl/Shift スナップ対応）
//! - Cmd+Z による Undo
//! - 確定済み線分・複製プレビュー・ドラッグ中プレビューの描画

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::EditCamera;
use crate::edit_layer;
use crate::grid::{draw_grid, snap_to_grid};
use crate::state::{FractalState, Line, UndoStack};

/// 線として確定するための最小長（これより短いドラッグは破棄）。
const MIN_LINE_LEN: f32 = 0.01;
const CONFIRMED_COLOR: Color = Color::srgb(0.9, 0.9, 1.0);
const PREVIEW_COLOR: Color = Color::srgb(1.0, 0.8, 0.4);
const SNAP_PREVIEW_COLOR: Color = Color::srgb(0.4, 1.0, 0.6);
const GRID_PREVIEW_COLOR: Color = Color::srgb(0.4, 0.8, 1.0);

/// Edit キャンバス専用のギズモグループ（Edit カメラのレンダーレイヤにのみ描画される）。
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct EditGizmos;

/// ドラッグ中の状態。確定済み線分は `FractalState` 側に格納し、
/// このリソースには進行中ドラッグの始点だけを保持する。
#[derive(Resource, Default)]
pub enum DrawState {
    #[default]
    Idle,
    Dragging {
        start: Vec2,
    },
}

/// Edit キャンバスの一連の機能（入力・描画・Undo・グリッド表示）をまとめて登録するプラグイン。
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

/// 修飾キーの押下状態を一括で読み出すためのまとめ。
struct Modifiers {
    /// Ctrl 押下中（グリッドスナップ）
    ctrl: bool,
    /// Shift 押下中（45° スナップ）
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

/// 始点から見た方向を 45° 単位にスナップした終点を返す（長さは元のまま保持）。
fn snap_to_45(start: Vec2, end: Vec2) -> Vec2 {
    let delta = end - start;
    if delta.length_squared() < f32::EPSILON {
        return end;
    }
    let angle = delta.y.atan2(delta.x);
    let snapped = (angle / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
    start + Vec2::new(snapped.cos(), snapped.sin()) * delta.length()
}

/// マウスカーソルの論理座標を Edit カメラのワールド座標へ変換する。
/// Edit ビューポート外なら `None`。
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

/// Cmd+Z で Undo スタックから 1 つ戻す。ドラッグ中であれば Idle に戻して整合性を保つ。
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

/// 左クリックドラッグによる線分入力を処理する。
/// 押下時に始点を確定、離した時に最小長を満たせば線分として `FractalState` に追加する。
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
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((cam, cam_tf)) = edit_cam.single() else {
        return;
    };
    let cursor = cursor_in_edit(window, cam, cam_tf);
    let modifiers = Modifiers::read(&keys);

    // egui がポインタを掴んでいるときは（パネル上クリックなど）描画を始めない
    let egui_wants_pointer = contexts
        .ctx_mut()
        .map(|ctx| ctx.is_using_pointer())
        .unwrap_or(false);

    if let Some(raw_start) = cursor
        && buttons.just_pressed(MouseButton::Left)
        && matches!(*draw_state, DrawState::Idle)
        && !egui_wants_pointer
    {
        let start = if modifiers.ctrl {
            snap_to_grid(raw_start)
        } else {
            raw_start
        };
        *draw_state = DrawState::Dragging { start };
    }

    if buttons.just_released(MouseButton::Left)
        && let DrawState::Dragging { start } = *draw_state
    {
        if let Some(raw_end) = cursor {
            let (end, _) = snap_endpoint(start, raw_end, &modifiers);
            if (end - start).length() >= MIN_LINE_LEN {
                undo_stack.push(state.clone());
                state.base_shape.lines.push(Line { a: start, b: end });
            }
        }
        *draw_state = DrawState::Idle;
    }
}

/// Edit キャンバス上の確定済み直線・グリッド・ドラッグプレビューを描画する。
fn draw_canvas(
    state: Res<FractalState>,
    draw_state: Res<DrawState>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    edit_cam: Query<(&Camera, &GlobalTransform), With<EditCamera>>,
    mut gizmos: Gizmos<EditGizmos>,
) {
    draw_confirmed_lines(&state, &mut gizmos);

    let modifiers = Modifiers::read(&keys);
    let cursor = windows
        .single()
        .ok()
        .zip(edit_cam.single().ok())
        .and_then(|(w, (cam, cam_tf))| cursor_in_edit(w, cam, cam_tf));

    if modifiers.ctrl {
        draw_grid(&mut gizmos, cursor.map(snap_to_grid));
    }

    if let DrawState::Dragging { start } = *draw_state
        && let Some(raw_end) = cursor
    {
        let (end, color) = snap_endpoint(start, raw_end, &modifiers);
        gizmos.line_2d(start, end, color);
    }
}

/// 確定済みの基図形の直線を描画する。
fn draw_confirmed_lines(state: &FractalState, gizmos: &mut Gizmos<EditGizmos>) {
    for line in &state.base_shape.lines {
        gizmos.line_2d(line.a, line.b, CONFIRMED_COLOR);
    }
}

