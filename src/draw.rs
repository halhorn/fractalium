use bevy::camera::Projection;
use bevy::color::Hsla;
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
const GRID_PREVIEW_COLOR: Color = Color::srgb(0.4, 0.8, 1.0);
const GRID_DOT_COLOR: Color = Color::srgba(0.6, 0.7, 1.0, 0.35);
const GRID_HIGHLIGHT_COLOR: Color = Color::srgba(0.4, 1.0, 0.6, 0.9);
const GRID_DOT_RADIUS: f32 = 0.012;
const GRID_HIGHLIGHT_RADIUS: f32 = 0.025;
const GRID_SQUARE_DIV: f32 = 8.0;
const GRID_TRI_DIV: f32 = 6.0;

pub struct DrawPlugin;

impl Plugin for DrawPlugin {
    fn build(&self, app: &mut App) {
        let mut edit_config = bevy::gizmos::config::GizmoConfig {
            render_layers: edit_layer(),
            ..Default::default()
        };
        edit_config.line.width = 2.0;

        let mut result_config = bevy::gizmos::config::GizmoConfig {
            render_layers: result_layer(),
            ..Default::default()
        };
        result_config.line.width = 2.0;

        app.init_resource::<DrawState>()
            .init_resource::<UndoStack>()
            .insert_gizmo_config(EditGizmos, edit_config)
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

fn snap_to_45(start: Vec2, end: Vec2) -> Vec2 {
    let delta = end - start;
    if delta.length_squared() < f32::EPSILON {
        return end;
    }
    let angle = delta.y.atan2(delta.x);
    let snapped = (angle / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
    start + Vec2::new(snapped.cos(), snapped.sin()) * delta.length()
}

fn nearest_square_grid_point(pos: Vec2) -> Vec2 {
    let s = 2.0 / GRID_SQUARE_DIV;
    Vec2::new((pos.x / s).round() * s, (pos.y / s).round() * s)
}

fn nearest_iso_grid_point(pos: Vec2) -> Vec2 {
    let h = 2.0 / GRID_TRI_DIV;
    let v = h * (3.0_f32.sqrt() / 2.0);
    let m_center = ((pos.y + 1.0) / v).round() as i32;
    let mut best = Vec2::ZERO;
    let mut best_dist = f32::MAX;
    for m in (m_center - 1)..=(m_center + 1) {
        let row_y = -1.0 + m as f32 * v;
        let offset = if m.rem_euclid(2) == 0 { 0.0 } else { h / 2.0 };
        let n_center = ((pos.x + 1.0 - offset) / h).round() as i32;
        for n in (n_center - 1)..=(n_center + 1) {
            let candidate = Vec2::new(-1.0 + n as f32 * h + offset, row_y);
            let dist = candidate.distance_squared(pos);
            if dist < best_dist {
                best_dist = dist;
                best = candidate;
            }
        }
    }
    best
}

fn snap_to_grid(pos: Vec2) -> Vec2 {
    let sq = nearest_square_grid_point(pos);
    let iso = nearest_iso_grid_point(pos);
    if iso.distance_squared(pos) <= sq.distance_squared(pos) {
        iso
    } else {
        sq
    }
}

fn draw_grid(gizmos: &mut Gizmos<EditGizmos>, highlighted: Option<Vec2>) {
    let sq_s = 2.0 / GRID_SQUARE_DIV;
    let tri_h = 2.0 / GRID_TRI_DIV;
    let tri_v = tri_h * (3.0_f32.sqrt() / 2.0);

    let n_sq = GRID_SQUARE_DIV as i32;
    for xi in 0..=n_sq {
        for yi in 0..=n_sq {
            let pt = Vec2::new(-1.0 + xi as f32 * sq_s, -1.0 + yi as f32 * sq_s);
            let is_highlight = highlighted.map_or(false, |h| h.distance_squared(pt) < 1e-5);
            if is_highlight {
                gizmos.circle_2d(pt, GRID_HIGHLIGHT_RADIUS, GRID_HIGHLIGHT_COLOR);
            } else {
                gizmos.circle_2d(pt, GRID_DOT_RADIUS, GRID_DOT_COLOR);
            }
        }
    }

    let m_max = (2.0 / tri_v).ceil() as i32;
    for m in 0..=m_max {
        let y = -1.0 + m as f32 * tri_v;
        if y > 1.01 {
            break;
        }
        let offset = if m.rem_euclid(2) == 0 { 0.0 } else { tri_h / 2.0 };
        let n_max = ((2.0 - offset) / tri_h).ceil() as i32;
        for n in 0..=n_max {
            let x = -1.0 + n as f32 * tri_h + offset;
            if x > 1.01 {
                continue;
            }
            let pt = Vec2::new(x, y);
            // Skip points already drawn by square grid
            let on_sq_grid = (pt.x / sq_s).fract().abs() < 1e-4
                && (pt.y / (2.0 / GRID_SQUARE_DIV)).fract().abs() < 1e-4;
            if on_sq_grid {
                continue;
            }
            let is_highlight = highlighted.map_or(false, |h| h.distance_squared(pt) < 1e-5);
            if is_highlight {
                gizmos.circle_2d(pt, GRID_HIGHLIGHT_RADIUS, GRID_HIGHLIGHT_COLOR);
            } else {
                gizmos.circle_2d(pt, GRID_DOT_RADIUS, GRID_DOT_COLOR);
            }
        }
    }
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

    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if buttons.just_pressed(MouseButton::Left)
        && matches!(*draw_state, DrawState::Idle)
        && !egui_wants_pointer
        && let Some(raw_start) = cursor
    {
        let start = if ctrl { snap_to_grid(raw_start) } else { raw_start };
        *draw_state = DrawState::Dragging { start };
    }

    if buttons.just_released(MouseButton::Left)
        && let DrawState::Dragging { start } = *draw_state
    {
        if let Some(raw_end) = cursor {
            let end = if ctrl {
                snap_to_grid(raw_end)
            } else if shift {
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

    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    let cursor = windows
        .single()
        .ok()
        .zip(edit_cam.single().ok())
        .and_then(|(w, (cam, cam_tf))| cursor_in_edit(w, cam, cam_tf));

    if ctrl {
        let highlighted = cursor.map(snap_to_grid);
        draw_grid(&mut gizmos, highlighted);
    }

    if let DrawState::Dragging { start } = *draw_state
        && let Some(raw_end) = cursor
    {
        let (end, color) = if ctrl {
            (snap_to_grid(raw_end), GRID_PREVIEW_COLOR)
        } else if shift {
            (snap_to_45(start, raw_end), SNAP_PREVIEW_COLOR)
        } else {
            (raw_end, PREVIEW_COLOR)
        };
        gizmos.line_2d(start, end, color);
    }
}

fn hue_to_color(hue: f32) -> Color {
    let lin = LinearRgba::from(Hsla::new(hue % 360.0, 0.88, 0.58, 1.0));
    Color::from(lin)
}

fn draw_fractal_result(state: Res<FractalState>, mut gizmos: Gizmos<ResultGizmos>) {
    let mut segments: Vec<(Vec2, Vec2, f32)> = Vec::new();
    // hue_step=360 means the first level divides the full hue wheel among its replicas.
    collect_fractal_segments(
        state.depth,
        Replica::identity(),
        &state.base_shape.lines,
        &state.replicas,
        &mut segments,
        0.0,
        360.0,
    );
    for &(a, b, hue) in &segments {
        gizmos.line_2d(a, b, hue_to_color(hue));
    }
}

/// Recursively collect leaf line segments with per-branch hue assignment.
///
/// `hue` is the base hue for this subtree. `hue_step` is the total hue range
/// allocated to this subtree; each replica sub-divides it evenly so that the
/// top-level replicas span the full 360° wheel and deeper levels narrow their
/// slice proportionally.
fn collect_fractal_segments(
    depth: u32,
    transform: Replica,
    lines: &[Line],
    replicas: &[Replica],
    out: &mut Vec<(Vec2, Vec2, f32)>,
    hue: f32,
    hue_step: f32,
) {
    if depth <= 1 {
        for line in lines {
            out.push((transform.apply(line.a), transform.apply(line.b), hue));
        }
        return;
    }
    let n = replicas.len() as f32;
    let child_step = hue_step / n;
    for (i, replica) in replicas.iter().enumerate() {
        collect_fractal_segments(
            depth - 1,
            transform.compose(*replica),
            lines,
            replicas,
            out,
            hue + i as f32 * child_step,
            child_step,
        );
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
