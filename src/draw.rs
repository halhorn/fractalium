use bevy::camera::Projection;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::EguiContexts;

use crate::state::{FractalState, Line, Replica, UndoStack};
use crate::{EditCamera, ResultCamera, edit_layer, result_layer};

/// Drag-in-progress state for the Edit canvas. Confirmed lines live in
/// `FractalState::base_shape.lines`; only the in-flight start point lives here.
#[derive(Resource, Default)]
pub enum DrawState {
    #[default]
    Idle,
    Dragging {
        start: Vec2,
    },
}

/// Gizmo group dedicated to the Edit canvas. Bound to `edit_layer()` so its
/// gizmos only appear in the Edit camera's viewport.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct EditGizmos;

/// Gizmo group dedicated to the Result canvas.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ResultGizmos;

const MIN_LINE_LEN: f32 = 0.01;
const CONFIRMED_COLOR: Color = Color::srgb(0.9, 0.9, 1.0);
const PREVIEW_COLOR: Color = Color::srgb(1.0, 0.8, 0.4);
const SNAP_PREVIEW_COLOR: Color = Color::srgb(0.4, 1.0, 0.6);
const FRACTAL_COLOR: Color = Color::srgb(0.85, 0.95, 0.9);

pub struct DrawPlugin;

impl Plugin for DrawPlugin {
    fn build(&self, app: &mut App) {
        let mut config = bevy::gizmos::config::GizmoConfig {
            render_layers: edit_layer(),
            ..Default::default()
        };
        config.line.width = 2.0;

        let result_config = bevy::gizmos::config::GizmoConfig {
            render_layers: result_layer(),
            ..Default::default()
        };

        app.init_resource::<DrawState>()
            .init_resource::<UndoStack>()
            .insert_gizmo_config(EditGizmos, config)
            .insert_gizmo_config(ResultGizmos, result_config)
            .add_systems(
                Update,
                (
                    handle_undo,
                    handle_drag_input,
                    draw_lines,
                    draw_replicas,
                    draw_fractal_result,
                    zoom_result,
                )
                    .chain(),
            );
    }
}

/// Convert the cursor's window position into the Edit canvas's normalized
/// [-1, 1] coordinate. Returns `None` when the cursor is outside the Edit
/// camera's viewport (or there is no cursor).
fn cursor_in_edit(
    window: &Window,
    edit_cam: &Camera,
    edit_cam_transform: &GlobalTransform,
) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    edit_cam
        .viewport_to_world_2d(edit_cam_transform, cursor)
        .ok()
}

/// Snap the endpoint so the direction from `start` to `end` is a multiple of 45°.
fn snap_to_45(start: Vec2, end: Vec2) -> Vec2 {
    let delta = end - start;
    if delta.length_squared() < f32::EPSILON {
        return end;
    }
    let angle = delta.y.atan2(delta.x);
    let snapped = (angle / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
    start + Vec2::new(snapped.cos(), snapped.sin()) * delta.length()
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
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((cam, cam_tf)) = edit_cam.single() else {
        return;
    };
    let cursor = cursor_in_edit(window, cam, cam_tf);

    let egui_wants_pointer = contexts
        .ctx_mut()
        .map(|ctx| ctx.wants_pointer_input())
        .unwrap_or(false);

    if buttons.just_pressed(MouseButton::Left)
        && matches!(*draw_state, DrawState::Idle)
        && !egui_wants_pointer
        && let Some(start) = cursor
    {
        *draw_state = DrawState::Dragging { start };
    }

    if buttons.just_released(MouseButton::Left)
        && let DrawState::Dragging { start } = *draw_state
    {
        if let Some(raw_end) = cursor {
            let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
            let end = if shift {
                snap_to_45(start, raw_end)
            } else {
                raw_end
            };
            if (end - start).length() >= MIN_LINE_LEN {
                undo_stack.push(state.clone());
                state.base_shape.lines.push(Line { a: start, b: end });
            }
        }
        *draw_state = DrawState::Idle;
    }
}

fn draw_lines(
    state: Res<FractalState>,
    draw_state: Res<DrawState>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    edit_cam: Query<(&Camera, &GlobalTransform), With<EditCamera>>,
    mut gizmos: Gizmos<EditGizmos>,
) {
    for line in &state.base_shape.lines {
        gizmos.line_2d(line.a, line.b, CONFIRMED_COLOR);
    }

    if let DrawState::Dragging { start } = *draw_state
        && let Ok(window) = windows.single()
        && let Ok((cam, cam_tf)) = edit_cam.single()
        && let Some(raw_end) = cursor_in_edit(window, cam, cam_tf)
    {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let (end, color) = if shift {
            (snap_to_45(start, raw_end), SNAP_PREVIEW_COLOR)
        } else {
            (raw_end, PREVIEW_COLOR)
        };
        gizmos.line_2d(start, end, color);
    }
}

fn draw_fractal_result(state: Res<FractalState>, mut gizmos: Gizmos<ResultGizmos>) {
    draw_fractal_recursive(
        state.depth,
        Replica::identity(),
        &state.base_shape.lines,
        &state.replicas,
        &mut gizmos,
    );
}

fn draw_fractal_recursive(
    depth: u32,
    transform: Replica,
    lines: &[Line],
    replicas: &[Replica],
    gizmos: &mut Gizmos<ResultGizmos>,
) {
    if depth <= 1 {
        for line in lines {
            gizmos.line_2d(
                transform.apply(line.a),
                transform.apply(line.b),
                FRACTAL_COLOR,
            );
        }
    } else {
        for replica in replicas {
            draw_fractal_recursive(
                depth - 1,
                transform.compose(*replica),
                lines,
                replicas,
                gizmos,
            );
        }
    }
}

fn draw_replicas(state: Res<FractalState>, mut gizmos: Gizmos<EditGizmos>) {
    let Some(bbox) = state.base_shape.bbox() else {
        return;
    };
    let corners = [
        Vec2::new(bbox.min.x, bbox.min.y),
        Vec2::new(bbox.max.x, bbox.min.y),
        Vec2::new(bbox.max.x, bbox.max.y),
        Vec2::new(bbox.min.x, bbox.max.y),
    ];
    let color = Color::srgba(0.4, 0.7, 1.0, 0.6);
    for replica in &state.replicas {
        let pts: [Vec2; 4] = corners.map(|c| replica.apply(c));
        gizmos.line_2d(pts[0], pts[1], color);
        gizmos.line_2d(pts[1], pts[2], color);
        gizmos.line_2d(pts[2], pts[3], color);
        gizmos.line_2d(pts[3], pts[0], color);
    }
}

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
    // Only zoom when cursor is inside the Result viewport.
    let cursor_in_result = window
        .cursor_position()
        .and_then(|p| cam.viewport_to_world_2d(cam_tf, p).ok())
        .is_some();
    if !cursor_in_result {
        return;
    }
    if let Projection::Orthographic(ref mut ortho) = *proj {
        ortho.scale = (ortho.scale * (1.0 - total * 0.1)).clamp(0.2, 8.0);
    }
}
