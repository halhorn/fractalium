//! ウィンドウ幅に応じた egui パネル配置。戻り値は Edit / Placement / Result 各カメラ用の論理矩形。

use bevy::prelude::{Commands, NextState};
use bevy_egui::egui;

use crate::app::export::{PreparedResultImage, ResultImageOutlet};
use crate::app::mode_state::AppScreen;
use crate::app::session::{
    FractalState, PlacementState, SnapGrid, UiLayout, UndoStack,
};
use crate::app::share::sync::ShareNavigation;
use crate::core::shape::Replica;
use crate::ui::canvas::seed::DrawState;
use crate::ui::feedback::toast::{DeferredToast, EguiToast};

use super::chrome::{
    app_title_bar_contents, app_title_panel_frame, show_canvas_block, step_glyph_button,
};
use super::global_bar::global_controls_bar;
use super::params::{draw_params_controls, draw_params_panel};
use super::seed_header::base_shape_header_buttons;

/// 幅 700px 以上: 左ペイン・中央 Result・右 Parameters。戻り値は (edit, placement, result) の論理矩形。
#[allow(clippy::too_many_arguments)] // 1 フレーム分のレイアウトに必要なリソースを束ねるだけの関数。
pub(crate) fn layout_wide(
    ctx: &egui::Context,
    commands: &mut Commands,
    _win_w: f32,
    win_h: f32,
    state: &mut FractalState,
    snap_grid: &mut SnapGrid,
    draw_state: &mut DrawState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
    toast: &mut EguiToast,
    deferred_toast: &mut DeferredToast,
    prepared_png: &mut PreparedResultImage,
    share_nav: &ShareNavigation,
    png_outlet: &ResultImageOutlet,
    next_app_screen: &mut NextState<AppScreen>,
) -> (egui::Rect, egui::Rect, egui::Rect) {
    // Right: Params panel
    let params_w = if ui_layout.params_collapsed {
        28.0_f32
    } else {
        240.0_f32
    };
    let right_resp = egui::SidePanel::right("params")
        .exact_width(params_w)
        .show(ctx, |ui| {
            draw_params_panel(ui, state, undo_stack, ui_layout);
        });
    let central_right_x = right_resp.response.rect.min.x;

    // Left: Seed + Placement
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
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    global_controls_bar(
                        ui,
                        commands,
                        state,
                        snap_grid,
                        draw_state,
                        placement,
                        undo_stack,
                        toast,
                        deferred_toast,
                        prepared_png,
                        share_nav,
                        png_outlet,
                        next_app_screen,
                    );
                });
            ui.separator();
            let edit_rect = show_canvas_block(ui, "Seed", |ui| {
                base_shape_header_buttons(ui, state, draw_state, undo_stack, false);
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

/// 幅 700px 未満（ナロー）: 上タイトル・中央 Result・下段 Edit/Placement・Parameters。
#[allow(clippy::too_many_arguments)] // ワイド版と同じ引数セット（ナロー用パネル順のみ異なる）。
pub(crate) fn layout_narrow(
    ctx: &egui::Context,
    commands: &mut Commands,
    win_w: f32,
    _win_h: f32,
    state: &mut FractalState,
    snap_grid: &mut SnapGrid,
    draw_state: &mut DrawState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
    toast: &mut EguiToast,
    deferred_toast: &mut DeferredToast,
    prepared_png: &mut PreparedResultImage,
    share_nav: &ShareNavigation,
    png_outlet: &ResultImageOutlet,
    next_app_screen: &mut NextState<AppScreen>,
) -> (egui::Rect, egui::Rect, egui::Rect) {
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
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    draw_params_controls(ui, state, undo_stack);
                });
        }
    });

    let bottom_resp = egui::TopBottomPanel::bottom("bottom_canvases")
        .frame(egui::Frame::default())
        .exact_height(panel_h)
        .show(ctx, |ui| {
            let half_w = ui.available_width() / 2.0;
            let mut edit_rect = egui::Rect::NOTHING;
            let mut placement_rect = egui::Rect::NOTHING;
            ui.columns(2, |cols| {
                cols[0].set_max_width(half_w);
                edit_rect = show_canvas_block(&mut cols[0], "Seed", |ui| {
                    base_shape_header_buttons(ui, state, draw_state, undo_stack, true);
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

    let global_resp = egui::TopBottomPanel::bottom("global_controls")
        .min_height(32.0)
        .default_height(40.0)
        .max_height(108.0)
        .show(ctx, |ui| {
            global_controls_bar(
                ui,
                commands,
                state,
                snap_grid,
                draw_state,
                placement,
                undo_stack,
                toast,
                deferred_toast,
                prepared_png,
                share_nav,
                png_outlet,
                next_app_screen,
            );
        });

    let (edit_egui_rect, placement_egui_rect) = bottom_resp.inner;

    let result_rect = egui::Rect::from_min_max(
        egui::pos2(0.0, title_resp.response.rect.max.y),
        egui::pos2(win_w, global_resp.response.rect.min.y),
    );

    (edit_egui_rect, placement_egui_rect, result_rect)
}
