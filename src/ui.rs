//! 右側のパラメータパネル（egui）と左側ペイン（Seed / Placement）の描画、
//! および各カメラへのビューポート分配を提供するモジュール。
//!
//! 幅 700px 以上: 左サイドパネル（Undo/Snap を含む操作バー + Edit + Placement）+ 中央 Result + 右パラメータパネル。
//! Result ウィンドウ左上に Depth（スライダー＋数値）と Show generations のトグルを重ね表示する。
//! 幅 700px 未満: 最上部タイトル + 中段 Result（上記オーバーレイを含む）
//!               + Undo/Snap グローバル操作バー + 下部 (Edit | Placement)
//!               + Parameters（折りたたみ／展開時は Result と干渉しない下部パネル）

use bevy::ecs::system::SystemParam;
use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::edit::DrawState;
use crate::fractal::{clamp_fractal_state_depth, max_depth_for_budget, result_replica_color};
use crate::fractal_presets::FractalPreset;
use crate::platform_handles::PlatformHandles;
use crate::seed_shape::BaseShapePreset;
use crate::share;
use crate::state::{
    CanvasLayout, FractalState, PendingResultCameraFit, PlacementDrag, PlacementState,
    REPLICA_SCALE_MAX, REPLICA_SCALE_MIN, Replica, ScreenRect, UiLayout, UndoStack,
};
use crate::result_export::{
    PreparedResultImage, PreparedResultImageState, RequestResultImageExport, ResultImageOutlet,
    deliver_prepared_result_png, ExportPhase,
};
use crate::share::ShareNavigation;
use crate::toast::{DeferredToast, EguiToast};
use crate::view::fit_result_camera_if_requested;
use crate::{EditCamera, PlacementCamera, ResultCamera};

/// `params_panel` の Bevy システム引数上限を超えないように、3 カメラ `Query` を 1 つの `SystemParam` に束ねる。
#[derive(SystemParam)]
struct ViewportCamerasMut<'w, 's> {
    edit_cam: Query<
        'w,
        's,
        &'static mut Camera,
        (
            With<EditCamera>,
            Without<PlacementCamera>,
            Without<ResultCamera>,
        ),
    >,
    placement_cam: Query<
        'w,
        's,
        &'static mut Camera,
        (
            With<PlacementCamera>,
            Without<EditCamera>,
            Without<ResultCamera>,
        ),
    >,
    result_cam: Query<
        'w,
        's,
        &'static mut Camera,
        (
            With<ResultCamera>,
            Without<EditCamera>,
            Without<PlacementCamera>,
        ),
    >,
}

/// ナローレイアウトの + / - 用。高さはインタラクト高さ、幅はワイド用より詰めてヘッダに収める。
fn step_glyph_button(ui: &mut egui::Ui, label: &'static str) -> egui::Response {
    let h = ui.spacing().interact_size.y;
    let w = (h + 14.0) * 0.75;
    ui.add_sized(egui::vec2(w, h), egui::Button::new(label).small())
}

/// Seed ブロックのヘッダ右側: **[Shape] [-] [Clear]**（ワイド・ナロー共通）。
fn base_shape_header_buttons(
    ui: &mut egui::Ui,
    state: &mut FractalState,
    draw_state: &mut DrawState,
    undo_stack: &mut UndoStack,
    minus_compact: bool,
) {
    ui.menu_button("Shape", |ui| {
        ui.set_min_width(160.0);
        for &preset in BaseShapePreset::ALL {
            if ui.button(preset.label()).clicked() {
                undo_stack.push(state.clone());
                state.base_shape.lines.extend(preset.lines());
                ui.close();
            }
        }
    });

    let can_del = matches!(*draw_state, DrawState::Selected(i) if i < state.base_shape.lines.len());
    if minus_compact {
        let minus = ui.add_enabled_ui(can_del, |ui| step_glyph_button(ui, "-"));
        if minus.inner.clicked()
            && let DrawState::Selected(idx) = *draw_state
        {
            undo_stack.push(state.clone());
            state.base_shape.lines.remove(idx);
            *draw_state = DrawState::Idle;
        }
    } else if ui
        .add_enabled(can_del, egui::Button::new("-").small())
        .clicked()
        && let DrawState::Selected(idx) = *draw_state
    {
        undo_stack.push(state.clone());
        state.base_shape.lines.remove(idx);
        *draw_state = DrawState::Idle;
    }

    if ui.small_button("Clear").clicked() {
        undo_stack.push(state.clone());
        state.base_shape.lines.clear();
        *draw_state = DrawState::Idle;
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeferredToast>()
            .init_resource::<EguiToast>()
            .init_resource::<PendingResultCameraFit>();

        app.add_systems(EguiPrimaryContextPass, params_panel);

        app.add_systems(
            EguiPrimaryContextPass,
            fit_result_camera_if_requested.after(params_panel),
        );
    }
}

fn params_panel(
    mut contexts: EguiContexts,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FractalState>,
    mut draw_state: ResMut<DrawState>,
    mut undo_stack: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut layout: ResMut<CanvasLayout>,
    mut ui_layout: ResMut<UiLayout>,
    mut toast: ResMut<EguiToast>,
    mut deferred_toast: ResMut<DeferredToast>,
    mut pending_result_fit: ResMut<PendingResultCameraFit>,
    mut prepared_png: ResMut<PreparedResultImage>,
    platform_handles: Res<PlatformHandles>,
    mut cameras: ViewportCamerasMut,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let share_nav = &platform_handles.share_navigation;
    let png_outlet = &platform_handles.result_png_outlet;
    deferred_toast.flush_async_to_message();
    if let Some(msg) = std::mem::take(&mut deferred_toast.message) {
        toast.show(ctx, msg);
    }
    let Ok(window) = windows.single() else {
        return Ok(());
    };
    let scale = window.scale_factor();
    let win_w = window.width();
    let win_h = window.height();
    let win_phys = UVec2::new(window.physical_width(), window.physical_height());

    let is_narrow = win_w < 700.0;

    let (edit_egui_rect, placement_egui_rect, result_egui_rect) = if is_narrow {
        layout_narrow(
            ctx,
            &mut commands,
            win_w,
            win_h,
            &mut state,
            &mut draw_state,
            &mut undo_stack,
            &mut placement,
            &mut ui_layout,
            &mut toast,
            &mut deferred_toast,
            &mut prepared_png,
            share_nav,
            png_outlet,
            &mut pending_result_fit,
        )
    } else {
        layout_wide(
            ctx,
            &mut commands,
            win_w,
            win_h,
            &mut state,
            &mut draw_state,
            &mut undo_stack,
            &mut placement,
            &mut ui_layout,
            &mut toast,
            &mut deferred_toast,
            &mut prepared_png,
            share_nav,
            png_outlet,
            &mut pending_result_fit,
        )
    };

    paint_result_corner_controls(ctx, result_egui_rect, &mut *state, &mut layout);
    toast.paint(ctx);
    if let Ok(mut cam) = cameras.edit_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(edit_egui_rect, scale, win_phys);
    }
    if let Ok(mut cam) = cameras.placement_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(placement_egui_rect, scale, win_phys);
    }
    if let Ok(mut cam) = cameras.result_cam.single_mut() {
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
    commands: &mut Commands,
    _win_w: f32,
    win_h: f32,
    state: &mut FractalState,
    draw_state: &mut DrawState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
    toast: &mut EguiToast,
    deferred_toast: &mut DeferredToast,
    prepared_png: &mut PreparedResultImage,
    share_nav: &ShareNavigation,
    png_outlet: &ResultImageOutlet,
    pending_result_fit: &mut PendingResultCameraFit,
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
                        draw_state,
                        placement,
                        undo_stack,
                        toast,
                        deferred_toast,
                        prepared_png,
                        share_nav,
                        png_outlet,
                        pending_result_fit,
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

// === ナローレイアウト（幅 700px 未満: 縦持ちモバイル向け）===

fn layout_narrow(
    ctx: &egui::Context,
    commands: &mut Commands,
    win_w: f32,
    _win_h: f32,
    state: &mut FractalState,
    draw_state: &mut DrawState,
    undo_stack: &mut UndoStack,
    placement: &mut PlacementState,
    ui_layout: &mut UiLayout,
    toast: &mut EguiToast,
    deferred_toast: &mut DeferredToast,
    prepared_png: &mut PreparedResultImage,
    share_nav: &ShareNavigation,
    png_outlet: &ResultImageOutlet,
    pending_result_fit: &mut PendingResultCameraFit,
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

    // 3. グローバル操作バー（Edit/Placement の直上）
    let global_resp = egui::TopBottomPanel::bottom("global_controls")
        .min_height(32.0)
        .default_height(40.0)
        .max_height(108.0)
        .show(ctx, |ui| {
            global_controls_bar(
                ui,
                commands,
                state,
                draw_state,
                placement,
                undo_stack,
                toast,
                deferred_toast,
                prepared_png,
                share_nav,
                png_outlet,
                pending_result_fit,
            );
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

/// Result 左上の Depth / Show generations オーバーレイの論理ピクセル矩形を `layout` に書き込む（Result ビュー入力の貫通防止用）。
fn paint_result_corner_controls(
    ctx: &egui::Context,
    result_rect: egui::Rect,
    state: &mut FractalState,
    layout: &mut CanvasLayout,
) {
    if result_rect.width() < 1.0 || result_rect.height() < 1.0 {
        layout.result_depth_controls_rect = None;
        return;
    }

    let pad = egui::vec2(14.0, 12.0);
    let pack = egui::Area::new(egui::Id::new("result_depth_generations_corner"))
        .order(egui::Order::Middle)
        .constrain_to(result_rect)
        .pivot(egui::Align2::LEFT_TOP)
        .current_pos(result_rect.min + pad)
        .show(ctx, |ui| {
            let framed = egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.label(egui::RichText::new("Depth").small())
                        .on_hover_text(
                            "メッシュの線分数（基図形の線数 × 末端または全世代の描画回数）と再帰ノード数から見積もり、\
その予算を超える深さは選べません（WASM ではより厳しめ）。",
                        );

                    let depth_cap = max_depth_for_budget(
                        state.base_shape.lines.len(),
                        state.replicas.len(),
                        state.show_all_generations,
                    );
                    let range = 1..=depth_cap;

                    let h = ui.spacing().interact_size.y;
                    const RESERVE_OTHER: f32 = 284.0;
                    let slider_w = (result_rect.width() - pad.x * 2.0 - RESERVE_OTHER).clamp(80.0, 220.0);

                    let mut depth = state.depth;
                    let slider_r = ui.add_sized(
                        egui::vec2(slider_w, h),
                        egui::Slider::new(&mut depth, range.clone()).show_value(false),
                    );
                    ui.add(egui::DragValue::new(&mut depth).range(range).speed(1.0));

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
                        clamp_fractal_state_depth(state);
                    }
                });
            });
            framed.response.rect
        });

    let egui_r = pack.inner.expand(4.0);
    layout.result_depth_controls_rect = Some(ScreenRect {
        min: Vec2::new(egui_r.min.x, egui_r.min.y),
        max: Vec2::new(egui_r.max.x, egui_r.max.y),
    });
}

/// Web Share（iOS 共有シート → X 等）に渡す本文。`Copy link` と同じ状態から URL を組み立てる。
fn share_sheet_text_for_image_export(state: &FractalState, share_nav: &ShareNavigation) -> Option<String> {
    let url = match share::encode_state(state) {
        Ok(token) => share::share_url_from_token(share_nav, &token).ok(),
        Err(_) => None,
    };
    let body = match url {
        Some(u) => format!("\n#fractalium\n{u}"),
        None => "\n#fractalium".to_string(),
    };
    Some(body)
}

/// undo / redo / snap を並べた操作バー（狭い幅では左側グループが折り返し）。Share は常にバー右端。
fn global_controls_bar(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    state: &mut FractalState,
    draw_state: &mut DrawState,
    placement: &mut PlacementState,
    undo_stack: &mut UndoStack,
    toast: &mut EguiToast,
    deferred_toast: &mut DeferredToast,
    prepared_png: &mut PreparedResultImage,
    share_nav: &ShareNavigation,
    png_outlet: &ResultImageOutlet,
    pending_result_fit: &mut PendingResultCameraFit,
) {
    ui.add_space(4.0);
    let row_h = ui.spacing().interact_size.y;
    ui.horizontal(|ui| {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(undo_stack.can_undo(), egui::Button::new("↩"))
                .clicked()
            {
                if let Some(prev) = undo_stack.undo_pop(state.clone()) {
                    *state = prev;
                }
            }
            if ui
                .add_enabled(undo_stack.can_redo(), egui::Button::new("↪"))
                .clicked()
            {
                if let Some(next) = undo_stack.redo_pop(state.clone()) {
                    *state = next;
                }
            }
            ui.add_space(6.0);

            let mut snap_btn = egui::Button::new("Snap");
            if state.snap_grid {
                snap_btn = snap_btn.fill(egui::Color32::from_rgb(60, 120, 60));
            }
            if ui.add(snap_btn).clicked() {
                state.snap_grid = !state.snap_grid;
            }

            ui.add_space(6.0);
            ui.menu_button("Preset", |ui| {
                ui.set_min_width(200.0);
                for &preset in FractalPreset::ALL {
                    if ui.button(preset.label()).clicked() {
                        undo_stack.push(state.clone());
                        let snap = state.snap_grid;
                        *state = preset.build();
                        state.snap_grid = snap;
                        clamp_fractal_state_depth(state);
                        *draw_state = DrawState::Idle;
                        placement.selected = None;
                        placement.drag = PlacementDrag::Idle;
                        pending_result_fit.0 = true;
                        ui.close();
                    }
                }
            });
        });

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width().max(0.0), row_h),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                let share_menu = ui.menu_button("Share", |ui| {
                    ui.set_min_width(220.0);
                    if ui.button("Copy link").clicked() {
                        match share::encode_state(state) {
                            Ok(token) => match share::share_url_from_token(share_nav, &token) {
                                Ok(url) => {
                                    ui.ctx().copy_text(url);
                                    toast.show(ui.ctx(), "Link copied");
                                }
                                Err(e) => bevy::log::warn!("share URL: {e}"),
                            },
                            Err(e) => bevy::log::warn!("share encode: {e}"),
                        }
                        ui.close();
                    }
                    let export_working = matches!(
                        prepared_png.export_phase,
                        ExportPhase::Warm(_) | ExportPhase::Capturing
                    );
                    let (dl_label, can_download) = match &prepared_png.state {
                        PreparedResultImageState::Ready { .. } => ("Download image", true),
                        PreparedResultImageState::Preparing => ("Preparing image…", false),
                        PreparedResultImageState::None => {
                            if export_working {
                                ("Preparing image…", false)
                            } else {
                                ("Download image", false)
                            }
                        }
                    };
                    if ui
                        .add_enabled(can_download, egui::Button::new(dl_label))
                        .clicked()
                    {
                        deliver_prepared_result_png(
                            png_outlet,
                            prepared_png,
                            deferred_toast,
                            share_sheet_text_for_image_export(state, share_nav),
                        );
                        ui.close();
                    }
                });
                let menu_open = share_menu.inner.is_some();
                if menu_open && !prepared_png.share_menu_was_open {
                    if matches!(prepared_png.export_phase, ExportPhase::Idle) {
                        prepared_png.state = PreparedResultImageState::Preparing;
                        commands.write_message(RequestResultImageExport);
                    }
                }
                prepared_png.share_menu_was_open = menu_open;
            },
        );
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

fn draw_params_controls(ui: &mut egui::Ui, state: &mut FractalState, undo_stack: &mut UndoStack) {
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
            position_row(ui, &mut replica.position);
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

fn position_row(ui: &mut egui::Ui, position: &mut Vec2) {
    ui.horizontal(|ui| {
        ui.label("TX");
        ui.add(
            egui::DragValue::new(&mut position.x)
                .speed(0.01)
                .range(-2.0..=2.0),
        );
        ui.label("TY");
        ui.add(
            egui::DragValue::new(&mut position.y)
                .speed(0.01)
                .range(-2.0..=2.0),
        );
    });
}

fn rotation_row(ui: &mut egui::Ui, rotation: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Rot (deg)");
        let mut deg = rotation.to_degrees();
        if ui
            .add(
                egui::DragValue::new(&mut deg)
                    .speed(0.5)
                    .range(-180.0..=180.0),
            )
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
