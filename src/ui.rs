//! 右側のパラメータパネル（egui）と左側ペイン（Base Shape / Placement）の描画、
//! および各カメラへのビューポート分配を提供するモジュール。
//!
//! 幅 700px 以上: 左サイドパネル (Edit + Placement) + 中央 Result + 右パラメータパネル
//! 幅 700px 未満: 上部パネル (Edit | Placement) + 中央 Result + 下部パラメータパネル

use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::fractal::result_replica_color;
use crate::state::{CanvasLayout, FractalState, PlacementState, Replica, UiLayout, UndoStack};
use crate::{EditCamera, PlacementCamera, ResultCamera};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, params_panel);
    }
}

fn params_panel(
    mut contexts: EguiContexts,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FractalState>,
    mut undo_stack: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut layout: ResMut<CanvasLayout>,
    mut ui_layout: ResMut<UiLayout>,
    buttons: Res<ButtonInput<MouseButton>>,
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
    let Ok(window) = windows.single() else { return Ok(()); };
    let scale = window.scale_factor();
    let win_w = window.width();
    let win_h = window.height();
    let win_phys = UVec2::new(window.physical_width(), window.physical_height());

    let is_narrow = win_w < 700.0;

    let (edit_egui_rect, placement_egui_rect, result_egui_rect) = if is_narrow {
        layout_narrow(
            ctx, win_w, win_h,
            &mut state, &mut undo_stack, &mut placement, &mut ui_layout, &buttons,
        )
    } else {
        layout_wide(
            ctx, win_w, win_h,
            &mut state, &mut undo_stack, &mut placement, &mut ui_layout, &buttons,
        )
    };

    if let Ok(mut cam) = edit_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(edit_egui_rect, scale, win_phys);
    }
    if let Ok(mut cam) = placement_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(placement_egui_rect, scale, win_phys);
    }
    if let Ok(mut cam) = result_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(result_egui_rect, scale, win_phys);
    }

    layout.placement_min_x = placement_egui_rect.min.x;
    layout.placement_max_x = placement_egui_rect.max.x;
    layout.placement_min_y = placement_egui_rect.min.y;
    layout.placement_max_y = placement_egui_rect.max.y;

    Ok(())
}

// === ワイドレイアウト（幅 700px 以上）===

fn layout_wide(
    ctx: &egui::Context,
    _win_w: f32,
    win_h: f32,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
    buttons: &ButtonInput<MouseButton>,
) -> (egui::Rect, egui::Rect, egui::Rect) {
    // Right: Params panel
    let collapsed = ui_layout.params_collapsed;
    let params_w = if collapsed { 28.0_f32 } else { 240.0_f32 };
    let right_resp = egui::SidePanel::right("params")
        .exact_width(params_w)
        .show(ctx, |ui| {
            if collapsed {
                if ui.button("▶").clicked() { ui_layout.params_collapsed = false; }
            } else {
                if ui.button("◀").clicked() { ui_layout.params_collapsed = true; }
                ui.separator();
                draw_params_controls(ui, state, undo_stack, placement, buttons);
            }
        });
    let central_right_x = right_resp.response.rect.min.x;

    // Left: Base Shape + Placement
    let side = (win_h / 3.0).max(80.0);
    let left_panel_w = side + 2.0 * 8.0 + 6.0;

    let left_resp = egui::SidePanel::left("left_pane")
        .frame(egui::Frame::default())
        .exact_width(left_panel_w)
        .show(ctx, |ui| {
            let edit_rect = show_canvas_block(ui, "Base Shape", |ui| {
                snap_toggle_buttons(ui, state);
                ui.separator();
                if ui.small_button("Clear").clicked() {
                    undo_stack.push(state.clone());
                    state.base_shape.lines.clear();
                }
            });
            ui.add_space(4.0);
            let placement_rect = show_canvas_block(ui, "Placement", |ui| {
                if ui.small_button("+ Add").clicked() {
                    undo_stack.push(state.clone());
                    placement.selected = Some(state.replicas.len());
                    state.replicas.push(Replica::default_new());
                }
            });
            (edit_rect, placement_rect)
        });

    let central_left_x = left_resp.response.rect.max.x;
    let (edit_egui_rect, placement_egui_rect) = left_resp.inner;

    let result_rect = egui::Rect::from_min_max(
        egui::pos2(central_left_x, 0.0),
        egui::pos2(central_right_x, win_h),
    );

    (edit_egui_rect, placement_egui_rect, result_rect)
}

// === ナローレイアウト（幅 700px 未満: 縦持ちモバイル向け）===

fn layout_narrow(
    ctx: &egui::Context,
    win_w: f32,
    _win_h: f32,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
    buttons: &ButtonInput<MouseButton>,
) -> (egui::Rect, egui::Rect, egui::Rect) {
    // Bottom: Params panel (collapsible)
    let params_inner_h = if ui_layout.params_collapsed { 0.0_f32 } else { 180.0_f32 };
    let params_panel_h = params_inner_h + 32.0; // ボタン行 + コンテンツ
    let bottom_resp = egui::TopBottomPanel::bottom("params_bottom")
        .exact_height(params_panel_h)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let label = if ui_layout.params_collapsed { "▲ Parameters" } else { "▼ Parameters" };
                if ui.button(label).clicked() {
                    ui_layout.params_collapsed = !ui_layout.params_collapsed;
                }
            });
            if !ui_layout.params_collapsed {
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    draw_params_controls(ui, state, undo_stack, placement, buttons);
                });
            }
        });

    // Top: Edit + Placement side by side
    let canvas_side = (win_w * 0.5 - 24.0).clamp(60.0, 280.0);
    let top_panel_h = canvas_side + 52.0;
    let top_resp = egui::TopBottomPanel::top("top_canvases")
        .frame(egui::Frame::default()) // パネルのデフォルト灰色背景を透明化
        .exact_height(top_panel_h)
        .show(ctx, |ui| {
            let half_w = ui.available_width() / 2.0;
            let mut edit_rect = egui::Rect::NOTHING;
            let mut placement_rect = egui::Rect::NOTHING;
            ui.columns(2, |cols| {
                cols[0].set_max_width(half_w);
                edit_rect = show_canvas_block(&mut cols[0], "Base Shape", |ui| {
                    snap_toggle_buttons(ui, state);
                });
                cols[1].set_max_width(half_w);
                placement_rect = show_canvas_block(&mut cols[1], "Placement", |ui| {
                    if ui.small_button("+").clicked() {
                        undo_stack.push(state.clone());
                        placement.selected = Some(state.replicas.len());
                        state.replicas.push(Replica::default_new());
                    }
                });
            });
            (edit_rect, placement_rect)
        });

    let (edit_egui_rect, placement_egui_rect) = top_resp.inner;

    let result_rect = egui::Rect::from_min_max(
        egui::pos2(0.0, top_resp.response.rect.max.y),
        egui::pos2(win_w, bottom_resp.response.rect.min.y),
    );

    (edit_egui_rect, placement_egui_rect, result_rect)
}

// === 共通 UI パーツ ===

/// グリッドスナップのトグルボタンを描画する。
fn snap_toggle_buttons(ui: &mut egui::Ui, state: &mut FractalState) {
    let mut btn = egui::Button::new("snap grid");
    if state.snap_grid {
        btn = btn.fill(egui::Color32::from_rgb(60, 120, 60));
    }
    if ui.add(btn).clicked() {
        state.snap_grid = !state.snap_grid;
    }
}

/// タイトルヘッダ＋キャンバス本体を持つブロックを描画し、本体の egui::Rect を返す。
fn show_canvas_block(
    ui: &mut egui::Ui,
    title: &str,
    header_buttons: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    canvas_block_frame()
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).heading());
                header_buttons(ui);
            });
            ui.separator();
            let s = ui.available_width();
            let (rect, _resp) = ui.allocate_exact_size(egui::Vec2::splat(s), egui::Sense::hover());
            rect
        })
        .inner
}

fn canvas_block_frame() -> egui::Frame {
    egui::Frame::default()
        .inner_margin(egui::Margin::same(8))
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 55, 72)))
}

/// egui 論理ピクセル Rect → Bevy 物理ピクセル Viewport。
fn egui_rect_to_viewport(rect: egui::Rect, scale: f32, win_phys: UVec2) -> Option<Viewport> {
    let x = ((rect.min.x * scale).round() as i32).max(0) as u32;
    let y = ((rect.min.y * scale).round() as i32).max(0) as u32;
    let w = ((rect.width() * scale).round() as i32).max(0) as u32;
    let h = ((rect.height() * scale).round() as i32).max(0) as u32;
    if x >= win_phys.x || y >= win_phys.y || w == 0 || h == 0 {
        return None;
    }
    let w = w.min(win_phys.x - x);
    let h = h.min(win_phys.y - y);
    if w == 0 || h == 0 {
        return None;
    }
    Some(Viewport {
        physical_position: UVec2::new(x, y),
        physical_size: UVec2::new(w, h),
        ..default()
    })
}

// === パラメータパネル内 UI ===

fn draw_params_controls(
    ui: &mut egui::Ui,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    buttons: &ButtonInput<MouseButton>,
) {
    ui.heading("Parameters");
    ui.separator();

    let mut depth = state.depth;
    let depth_resp = ui.add(egui::Slider::new(&mut depth, 1..=12).text("Depth"));
    let egui_stuck = depth_resp.dragged() && !buttons.pressed(MouseButton::Left);
    if depth_resp.changed() && !egui_stuck {
        state.depth = depth;
    }

    ui.checkbox(&mut state.show_all_generations, "Show all generations");
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
