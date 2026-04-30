//! 右側のパラメータパネル（egui）と、それに合わせたカメラのビューポート分配、
//! 各パネルへのオーバーレイ UI を提供するモジュール。

use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::fractal::result_replica_color;
use crate::state::{CanvasLayout, FractalState, PlacementState, Replica, UndoStack};
use crate::{EditCamera, PlacementCamera, ResultCamera};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, params_panel);
    }
}

/// 右ドックにパラメータパネルを描画し、ビューポートを割り当て、各パネルのオーバーレイ UI を描画する。
fn params_panel(
    mut contexts: EguiContexts,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FractalState>,
    mut undo_stack: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut layout: ResMut<CanvasLayout>,
    mut edit_cam: Query<
        &mut Camera,
        (With<EditCamera>, Without<PlacementCamera>, Without<ResultCamera>),
    >,
    mut placement_cam: Query<
        &mut Camera,
        (With<PlacementCamera>, Without<EditCamera>, Without<ResultCamera>),
    >,
    mut result_cam: Query<
        &mut Camera,
        (With<ResultCamera>, Without<EditCamera>, Without<PlacementCamera>),
    >,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let panel_logical_w = egui::SidePanel::right("params")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            draw_params_controls(ui, &mut state, &mut undo_stack, &mut placement);
        })
        .response
        .rect
        .width();

    // ビューポート計算とレイアウト情報の更新
    let Ok(window) = windows.single() else {
        return Ok(());
    };
    let scale = window.scale_factor();
    let panel_phys = (panel_logical_w * scale).round() as u32;
    let win_w = window.physical_width();
    let win_h = window.physical_height();

    if win_w > panel_phys && win_h > 0 {
        let canvas_w = win_w - panel_phys;
        // 左パネル（Edit / Placement）は 1/4 幅に絞り、Result を最大化する
        let left_w = (canvas_w / 4).max(120);
        let right_w = (canvas_w - left_w).max(1);
        let top_h = win_h / 2;
        let bot_h = (win_h - top_h).max(1);

        let left_w_log = left_w as f32 / scale;
        let top_h_log = top_h as f32 / scale;
        layout.left_w_logical = left_w_log;
        layout.top_h_logical = top_h_log;

        // カメラへのビューポート書き込み
        let edit_vp = Viewport {
            physical_position: UVec2::new(0, 0),
            physical_size: UVec2::new(left_w, top_h.max(1)),
            ..default()
        };
        let placement_vp = Viewport {
            physical_position: UVec2::new(0, top_h),
            physical_size: UVec2::new(left_w, bot_h),
            ..default()
        };
        let result_vp = Viewport {
            physical_position: UVec2::new(left_w, 0),
            physical_size: UVec2::new(right_w, win_h),
            ..default()
        };
        if let Ok(mut cam) = edit_cam.single_mut() {
            cam.viewport = Some(edit_vp);
        }
        if let Ok(mut cam) = placement_cam.single_mut() {
            cam.viewport = Some(placement_vp);
        }
        if let Ok(mut cam) = result_cam.single_mut() {
            cam.viewport = Some(result_vp);
        }

        draw_panel_overlays(ctx, left_w_log, top_h_log, &mut state, &mut undo_stack, &mut placement);
    }

    Ok(())
}

/// 各パネルのタイトルラベルと Edit パネルの操作ボタンを描画する。
fn draw_panel_overlays(
    ctx: &egui::Context,
    left_w: f32,
    top_h: f32,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
) {
    let title_color = egui::Color32::from_rgba_unmultiplied(190, 190, 210, 200);

    // ── Base Shape パネル（左上）タイトル ──
    egui::Area::new("title_edit".into())
        .fixed_pos(egui::pos2(6.0, 4.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Base Shape").color(title_color).small());
        });

    // ── Base Shape パネル：Clear Lines ボタン（左上パネルの下端） ──
    egui::Area::new("edit_clear_lines".into())
        .fixed_pos(egui::pos2(6.0, top_h - 30.0))
        .show(ctx, |ui| {
            if ui.button("Clear lines").clicked() {
                undo_stack.push(state.clone());
                state.base_shape.lines.clear();
            }
        });

    // ── Placement パネル（左下）タイトル ──
    egui::Area::new("title_placement".into())
        .fixed_pos(egui::pos2(6.0, top_h + 4.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Placement").color(title_color).small());
        });

    // ── Placement パネル：+ Add Replica ボタン ──
    egui::Area::new("placement_add_replica".into())
        .fixed_pos(egui::pos2(6.0, top_h + 22.0))
        .show(ctx, |ui| {
            if ui.button("+ Add replica").clicked() {
                undo_stack.push(state.clone());
                placement.selected = Some(state.replicas.len());
                state.replicas.push(Replica::default_new());
            }
        });

    // ── Result パネル（右）タイトル ──
    egui::Area::new("title_result".into())
        .fixed_pos(egui::pos2(left_w + 6.0, 4.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Result").color(title_color).small());
        });
}

// === パラメータパネル内 UI ===

fn draw_params_controls(ui: &mut egui::Ui, state: &mut FractalState, undo_stack: &mut UndoStack, placement: &mut PlacementState) {
    ui.heading("Parameters");
    ui.separator();
    ui.add(egui::Slider::new(&mut state.depth, 1..=12).text("Depth"));
    ui.separator();

    ui.label(format!("Lines: {}", state.base_shape.lines.len()));
    if ui.button("Clear lines").clicked() {
        undo_stack.push(state.clone());
        state.base_shape.lines.clear();
    }
    ui.separator();

    ui.label(format!("Replicas: {}", state.replicas.len()));
    if ui.button("+ Add replica").clicked() {
        undo_stack.push(state.clone());
        placement.selected = Some(state.replicas.len());
        state.replicas.push(Replica::default_new());
    }
    ui.separator();

    draw_replica_section(ui, state, undo_stack);
}

fn draw_replica_section(ui: &mut egui::Ui, state: &mut FractalState, undo_stack: &mut UndoStack) {
    let total = state.replicas.len();
    let mut to_delete: Option<usize> = None;
    for (i, replica) in state.replicas.iter_mut().enumerate() {
        if draw_replica_editor(ui, i, total, replica) {
            to_delete = Some(i);
        }
    }
    if let Some(i) = to_delete {
        undo_stack.push(state.clone());
        state.replicas.remove(i);
    }
}

fn draw_replica_editor(ui: &mut egui::Ui, i: usize, total: usize, replica: &mut Replica) -> bool {
    let title = egui::RichText::new(format!("Replica {i}"))
        .color(replica_title_color(i, total))
        .strong();
    let mut delete_requested = false;
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            translation_row(ui, &mut replica.translation);
            rotation_row(ui, &mut replica.rotation);
            scale_row(ui, &mut replica.scale);
            if ui.button("Delete this replica").clicked() {
                delete_requested = true;
            }
        });
    delete_requested
}

fn replica_title_color(i: usize, total: usize) -> egui::Color32 {
    let lin = result_replica_color(i, total);
    egui::Color32::from_rgb(
        (lin.red.powf(1.0 / 2.2) * 255.0) as u8,
        (lin.green.powf(1.0 / 2.2) * 255.0) as u8,
        (lin.blue.powf(1.0 / 2.2) * 255.0) as u8,
    )
}

fn translation_row(ui: &mut egui::Ui, translation: &mut Vec2) {
    ui.horizontal(|ui| {
        ui.label("TX");
        ui.add(egui::DragValue::new(&mut translation.x).speed(0.01).range(-2.0..=2.0));
        ui.label("TY");
        ui.add(egui::DragValue::new(&mut translation.y).speed(0.01).range(-2.0..=2.0));
    });
}

fn rotation_row(ui: &mut egui::Ui, rotation: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Rot (deg)");
        let mut deg = rotation.to_degrees();
        if ui
            .add(egui::DragValue::new(&mut deg).speed(0.5).range(-180.0..=180.0))
            .changed()
        {
            *rotation = deg.to_radians();
        }
    });
}

fn scale_row(ui: &mut egui::Ui, scale: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Scale");
        ui.add(egui::DragValue::new(scale).speed(0.005).range(0.05..=2.0));
    });
}
