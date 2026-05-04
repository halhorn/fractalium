//! 右側のパラメータパネル（egui）と左側ペイン（Base Shape / Placement）の描画、
//! および各カメラへのビューポート分配を提供するモジュール。
//!
//! 幅 700px 以上: 左サイドパネル（Undo/Snap を含む操作バー + Edit + Placement）+ 中央 Result + 右パラメータパネル。
//! Result ウィンドウ右下に Depth（スライダー＋数値）と Show generations のトグルボタンがある。
//! 幅 700px 未満: 最上部タイトル + 中段 Result（Depth / Show generations は Result 右下に重ね表示）
//!               + Undo/Snap グローバル操作バー + 下部 (Edit | Placement)
//!               + Parameters（折りたたみ／展開時は Result と干渉しない下部パネル）

use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::edit::DrawState;
use crate::fractal::result_replica_color;
use crate::state::{
    CanvasLayout, FractalState, PlacementState, Replica, UiLayout, UndoStack, REPLICA_SCALE_MAX,
    REPLICA_SCALE_MIN,
};
use crate::{EditCamera, PlacementCamera, ResultCamera};

/// ナローレイアウトの + / - 用。インタラクト高さに対して少しだけ幅を足す。
fn step_glyph_button(ui: &mut egui::Ui, label: &'static str) -> egui::Response {
    let h = ui.spacing().interact_size.y;
    let w = h + 14.0;
    ui.add_sized(egui::vec2(w, h), egui::Button::new(label).small())
}

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
    mut draw_state: ResMut<DrawState>,
    mut undo_stack: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut layout: ResMut<CanvasLayout>,
    mut ui_layout: ResMut<UiLayout>,
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
            ctx,
            win_w,
            win_h,
            &mut state,
            &mut draw_state,
            &mut undo_stack,
            &mut placement,
            &mut ui_layout,
        )
    } else {
        layout_wide(
            ctx,
            win_w,
            win_h,
            &mut state,
            &mut draw_state,
            &mut undo_stack,
            &mut placement,
            &mut ui_layout,
        )
    };

    paint_result_corner_controls(ctx, result_egui_rect, &mut *state);

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
    draw_state: &mut DrawState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
) -> (egui::Rect, egui::Rect, egui::Rect) {
    // Right: Params panel
    let params_w = if ui_layout.params_collapsed { 28.0_f32 } else { 240.0_f32 };
    let right_resp = egui::SidePanel::right("params")
        .exact_width(params_w)
        .show(ctx, |ui| {
            draw_params_panel(ui, state, undo_stack, ui_layout);
        });
    let central_right_x = right_resp.response.rect.min.x;

    // Left: Base Shape + Placement
    let side = (win_h / 3.0).max(80.0);
    let left_panel_w = side + 2.0 * 8.0 + 6.0;

    let left_resp = egui::SidePanel::left("left_pane")
        .frame(egui::Frame::default())
        .exact_width(left_panel_w)
        .show(ctx, |ui| {
            app_title_panel_frame().show(ui, |ui| {
                app_title_bar_contents(ui);
            });
            ui.separator();
            global_controls_bar(ui, state, undo_stack);
            ui.separator();
            let edit_rect = show_canvas_block(ui, "Base Shape", |ui| {
                let can_del = matches!(*draw_state, DrawState::Selected(i) if i < state.base_shape.lines.len());
                let minus = ui.add_enabled(can_del, egui::Button::new("-").small());
                if minus.clicked() {
                    if let DrawState::Selected(idx) = *draw_state {
                        undo_stack.push(state.clone());
                        state.base_shape.lines.remove(idx);
                        *draw_state = DrawState::Idle;
                    }
                }
                if ui.small_button("Clear").clicked() {
                    undo_stack.push(state.clone());
                    state.base_shape.lines.clear();
                    *draw_state = DrawState::Idle;
                }
            });
            ui.add_space(4.0);
            let placement_rect = show_canvas_block(ui, "Placement", |ui| {
                if ui.small_button("+ Add").clicked() {
                    undo_stack.push(state.clone());
                    placement.selected = Some(state.replicas.len());
                    state.replicas.push(Replica::default_new());
                }
                let can_del = placement.selected.is_some_and(|i| i < state.replicas.len());
                let minus = ui.add_enabled(can_del, egui::Button::new("-").small());
                if minus.clicked() {
                    if let Some(i) = placement.selected {
                        undo_stack.push(state.clone());
                        state.replicas.remove(i);
                        placement.selected = None;
                    }
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
    draw_state: &mut DrawState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
) -> (egui::Rect, egui::Rect, egui::Rect) {
    // Edit/Placement と同じ高さ基準を先に計算
    let canvas_side = (win_w * 0.5 - 24.0).clamp(60.0, 280.0);
    let panel_h = canvas_side + 52.0;

    let title_resp = egui::TopBottomPanel::top("app_title")
        .frame(app_title_panel_frame())
        .default_height(40.0)
        .min_height(34.0)
        .max_height(52.0)
        .show(ctx, |ui| {
            app_title_bar_contents(ui);
        });

    // 1. Params panel（最下部：折りたたみ時はタブ行、展開時は下から押し上げ）
    //    bottom パネルは先に定義したものが最下位に積まれる
    let params_panel = egui::TopBottomPanel::bottom("params");
    let params_panel = if ui_layout.params_collapsed {
        params_panel
    } else {
        params_panel.exact_height(panel_h)
    };
    params_panel.show(ctx, |ui| {
        let header_h = ui.spacing().interact_size.y.max(36.0);
        let header_clicked = ui
            .add_sized(
                egui::vec2(ui.available_width(), header_h),
                egui::Button::new(egui::RichText::new("Parameters").heading()).frame(false),
            )
            .clicked();
        if header_clicked {
            ui_layout.params_collapsed = !ui_layout.params_collapsed;
        }

        if !ui_layout.params_collapsed {
            ui.separator();
            let scroll_h = (panel_h - 48.0).max(40.0);
            egui::ScrollArea::vertical()
                .max_height(scroll_h)
                .show(ui, |ui| {
                    draw_params_controls(ui, state, undo_stack);
                });
        }
    });

    // 2. Edit + Placement キャンバス（params の直上）
    let bottom_resp = egui::TopBottomPanel::bottom("bottom_canvases")
        .frame(egui::Frame::default())
        .exact_height(panel_h)
        .show(ctx, |ui| {
            let half_w = ui.available_width() / 2.0;
            let mut edit_rect = egui::Rect::NOTHING;
            let mut placement_rect = egui::Rect::NOTHING;
            ui.columns(2, |cols| {
                cols[0].set_max_width(half_w);
                edit_rect = show_canvas_block(&mut cols[0], "Base Shape", |ui| {
                    let can_del =
                        matches!(*draw_state, DrawState::Selected(i) if i < state.base_shape.lines.len());
                    let minus = ui.add_enabled_ui(can_del, |ui| step_glyph_button(ui, "-"));
                    if minus.inner.clicked() {
                        if let DrawState::Selected(idx) = *draw_state {
                            undo_stack.push(state.clone());
                            state.base_shape.lines.remove(idx);
                            *draw_state = DrawState::Idle;
                        }
                    }
                });
                cols[1].set_max_width(half_w);
                placement_rect = show_canvas_block(&mut cols[1], "Placement", |ui| {
                    if step_glyph_button(ui, "+").clicked() {
                        undo_stack.push(state.clone());
                        placement.selected = Some(state.replicas.len());
                        state.replicas.push(Replica::default_new());
                    }
                    let can_del = placement.selected.is_some_and(|i| i < state.replicas.len());
                    let minus = ui.add_enabled_ui(can_del, |ui| step_glyph_button(ui, "-"));
                    if minus.inner.clicked() {
                        if let Some(i) = placement.selected {
                            undo_stack.push(state.clone());
                            state.replicas.remove(i);
                            placement.selected = None;
                        }
                    }
                });
            });
            (edit_rect, placement_rect)
        });

    // 3. グローバル操作バー（Edit/Placement の直上）
    let global_resp = egui::TopBottomPanel::bottom("global_controls")
        .min_height(32.0)
        .default_height(40.0)
        .max_height(108.0)
        .show(ctx, |ui| {
            global_controls_bar(ui, state, undo_stack);
        });

    let (edit_egui_rect, placement_egui_rect) = bottom_resp.inner;

    // 4. Result: タイトルより下・グローバル操作バーより上（全幅）
    let result_rect = egui::Rect::from_min_max(
        egui::pos2(0.0, title_resp.response.rect.max.y),
        egui::pos2(win_w, global_resp.response.rect.min.y),
    );

    (edit_egui_rect, placement_egui_rect, result_rect)
}

// === 共通 UI パーツ ===

fn app_title_panel_frame() -> egui::Frame {
    egui::Frame::default()
        .inner_margin(egui::Margin::symmetric(10, 8))
        .fill(egui::Color32::from_rgb(26, 26, 34))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 55, 72)))
}

fn app_title_bar_contents(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.label(egui::RichText::new("Fractalium").heading().strong());
    });
}

/// Result の右下へ重ねる Depth（スライダー＋数値）と「Show generations」トグル（Snap と同種のボタン）。
fn paint_result_corner_controls(
    ctx: &egui::Context,
    result_rect: egui::Rect,
    state: &mut FractalState,
) {
    if result_rect.width() < 1.0 || result_rect.height() < 1.0 {
        return;
    }

    let pad = egui::vec2(14.0, 12.0);
    let pivot_pos = result_rect.max - pad;

    egui::Area::new(egui::Id::new("result_depth_generations_corner"))
        .order(egui::Order::Middle)
        .constrain_to(result_rect)
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .current_pos(pivot_pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                // shrink-wrap の Area で available_width と wrap を使うとスライダー幅が崩れつまみ位置と数値がずれるため、
                // 1 行の horizontal と result_rect 基準の幅だけ使う。
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(egui::RichText::new("Depth").small());

                    let h = ui.spacing().interact_size.y;
                    // ラベル・DragValue・「Show generations」・余白ぶんを result 幅から除外
                    const RESERVE_OTHER: f32 = 284.0;
                    let slider_w = (result_rect.width() - pad.x * 2.0 - RESERVE_OTHER).clamp(80.0, 220.0);

                    let mut depth = state.depth;
                    let slider_r = ui.add_sized(
                        egui::vec2(slider_w, h),
                        egui::Slider::new(&mut depth, 1..=12).show_value(false),
                    );
                    ui.add(egui::DragValue::new(&mut depth).range(1..=12).speed(1.0));

                    // Bevy の LMB だけだとタッチ／トラックパッドとの齟齬で常にブロックすることがある。
                    // スライダーの幽霊ドラッグだけ egui の primary_down で抑止する。
                    let pointer_down = ctx.input(|i| i.pointer.primary_down());
                    let phantom_slider = slider_r.dragged() && !pointer_down;
                    if depth != state.depth && !phantom_slider {
                        state.depth = depth;
                    }

                    let mut gen_btn = egui::Button::new("Show generations");
                    if state.show_all_generations {
                        gen_btn = gen_btn.fill(egui::Color32::from_rgb(60, 120, 60));
                    }
                    if ui.add(gen_btn).clicked() {
                        state.show_all_generations = !state.show_all_generations;
                    }
                });
            });
        });
}

/// undo / redo / snap を並べた操作バー（狭い幅では折り返し）。Depth / generations は Result 右下へ表示。
fn global_controls_bar(
    ui: &mut egui::Ui,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
) {
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        // Undo / Redo
        if ui.add_enabled(undo_stack.can_undo(), egui::Button::new("↩")).clicked() {
            if let Some(prev) = undo_stack.undo_pop(state.clone()) {
                *state = prev;
            }
        }
        if ui.add_enabled(undo_stack.can_redo(), egui::Button::new("↪")).clicked() {
            if let Some(next) = undo_stack.redo_pop(state.clone()) {
                *state = next;
            }
        }
        ui.add_space(6.0);

        // Snap grid
        let mut snap_btn = egui::Button::new("Snap");
        if state.snap_grid {
            snap_btn = snap_btn.fill(egui::Color32::from_rgb(60, 120, 60));
        }
        if ui.add(snap_btn).clicked() {
            state.snap_grid = !state.snap_grid;
        }
    });
}

/// 折りたたみ状態を含むパラメータパネルの中身を描画する共通関数（wide / narrow 共用）。
fn draw_params_panel(
    ui: &mut egui::Ui,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
    ui_layout: &mut UiLayout,
) {
    if ui_layout.params_collapsed {
        if ui.button("▶").clicked() {
            ui_layout.params_collapsed = false;
        }
    } else {
        ui.horizontal(|ui| {
            if ui.button("◀").clicked() {
                ui_layout.params_collapsed = true;
            }
            ui.heading("Parameters");
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            draw_params_controls(ui, state, undo_stack);
        });
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
) {
    ui.label(format!("Lines: {}", state.base_shape.lines.len()));
    ui.add_space(6.0);

    ui.label(format!("Replicas: {}", state.replicas.len()));
    ui.add_space(6.0);

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
        ui.add(
            egui::DragValue::new(scale)
                .speed(0.005)
                .range(REPLICA_SCALE_MIN..=REPLICA_SCALE_MAX),
        );
    });
}
