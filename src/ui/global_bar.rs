//! Undo / Redo / Snap / Preset / Share のワイド・ナロー共通ツールバー。

use bevy::prelude::Commands;
use bevy_egui::egui;

use crate::app::{encode_state, replace_fractal_state_keep_snap, share_sheet_text_for_export};
use crate::edit::DrawState;
use crate::fractal_presets::FractalPreset;
use crate::result_export::{
    PreparedResultImage, PreparedResultImageState, RequestResultImageExport, ResultImageOutlet,
    deliver_prepared_result_png, ExportPhase,
};
use crate::share::{share_url_from_token, ShareNavigation};
use crate::state::{
    FractalState, PendingResultCameraFit, PlacementDrag, PlacementState, UndoStack,
};
use crate::toast::{DeferredToast, EguiToast};

/// undo / redo / snap を並べた操作バー（狭い幅では左側グループが折り返し）。Share は常にバー右端。
#[allow(clippy::too_many_arguments)] // wide / narrow の `layout_*` と同じリソースをそのまま受け渡す。
pub(crate) fn global_controls_bar(
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
                        replace_fractal_state_keep_snap(state, preset.build());
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
                        match encode_state(state) {
                            Ok(token) => match share_url_from_token(share_nav, &token) {
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
                            share_sheet_text_for_export(state, &share_nav.0),
                        );
                        ui.close();
                    }
                });
                let menu_open = share_menu.inner.is_some();
                if menu_open
                    && !prepared_png.share_menu_was_open
                    && matches!(prepared_png.export_phase, ExportPhase::Idle)
                {
                    prepared_png.state = PreparedResultImageState::Preparing;
                    commands.write_message(RequestResultImageExport);
                }
                prepared_png.share_menu_was_open = menu_open;
            },
        );
    });
}
