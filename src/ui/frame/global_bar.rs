//! Undo / Redo / Snap / Open / Share のワイド・ナロー共通ツールバー。

use bevy::prelude::{Commands, NextState};
use bevy_egui::egui;

use crate::analytics;
use crate::app::export::{
    ExportPhase, PreparedResultImage, PreparedResultImageState, RequestResultImageExport,
    ResultImageOutlet, deliver_prepared_result_png,
};
use crate::app::mode_state::AppScreen;
use crate::app::session::{FractalState, PlacementState, SnapGrid, UndoStack};
use crate::app::share::payload::{encode_state, share_sheet_text_for_export};
use crate::app::share::sync::{ShareNavigation, share_url_from_token};
use crate::ui::canvas::seed::DrawState;
use crate::ui::feedback::toast::{DeferredToast, EguiToast};

/// GA4 カスタムイベント名（Undo）。
const GA4_EVT_UNDO: &str = "fractalium_undo";
/// GA4 カスタムイベント名（Redo）。
const GA4_EVT_REDO: &str = "fractalium_redo";
/// GA4 カスタムイベント名（Snap グリッドのオン／オフ）。
const GA4_EVT_SNAP_TOGGLE: &str = "fractalium_snap_toggle";
/// GA4 カスタムイベント名（プリセット一覧を開く）。
const GA4_EVT_OPEN_PRESET_PICKER: &str = "fractalium_open_preset_picker";
/// GA4 カスタムイベント名（共有リンクをコピー）。
const GA4_EVT_SHARE_COPY_LINK: &str = "fractalium_share_copy_link";
/// GA4 カスタムイベント名（結果 PNG をダウンロード）。
const GA4_EVT_SHARE_DOWNLOAD_IMAGE: &str = "fractalium_share_download_image";
/// GA4 イベントパラメータキー（共有ページの完全 URL）。
const GA4_PARAM_SHARE_URL: &str = "share_url";
/// GA4 イベントパラメータキー（`0` / `1` = オフ／オン）。
const GA4_PARAM_ENABLED: &str = "enabled";

/// undo / redo / snap を左へ並べる（狭い幅では折り返し）。Open と Share は右寄せで Share が最右端。
#[allow(clippy::too_many_arguments)] // wide / narrow の `layout_*` と同じリソースをそのまま受け渡す。
pub(crate) fn global_controls_bar(
    ui: &mut egui::Ui,
    commands: &mut Commands,
    state: &mut FractalState,
    snap_grid: &mut SnapGrid,
    _draw_state: &mut DrawState,
    _placement: &mut PlacementState,
    undo_stack: &mut UndoStack,
    toast: &mut EguiToast,
    deferred_toast: &mut DeferredToast,
    prepared_png: &mut PreparedResultImage,
    share_nav: &ShareNavigation,
    png_outlet: &ResultImageOutlet,
    next_app_screen: &mut NextState<AppScreen>,
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
                analytics::track_event(GA4_EVT_UNDO, &[]);
            }
            if ui
                .add_enabled(undo_stack.can_redo(), egui::Button::new("↪"))
                .clicked()
            {
                if let Some(next) = undo_stack.redo_pop(state.clone()) {
                    *state = next;
                }
                analytics::track_event(GA4_EVT_REDO, &[]);
            }
            ui.add_space(6.0);

            let mut snap_btn = egui::Button::new("Snap");
            if snap_grid.0 {
                snap_btn = snap_btn.fill(egui::Color32::from_rgb(60, 120, 60));
            }
            if ui.add(snap_btn).clicked() {
                snap_grid.0 = !snap_grid.0;
                analytics::track_event(
                    GA4_EVT_SNAP_TOGGLE,
                    &[(
                        GA4_PARAM_ENABLED,
                        if snap_grid.0 { "1" } else { "0" },
                    )],
                );
            }
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
                                    analytics::track_event(
                                        GA4_EVT_SHARE_COPY_LINK,
                                        &[(GA4_PARAM_SHARE_URL, url.as_str())],
                                    );
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
                        let share_url = encode_state(state)
                            .ok()
                            .and_then(|t| share_url_from_token(share_nav, &t).ok());
                        match share_url.as_ref() {
                            Some(u) => {
                                analytics::track_event(
                                    GA4_EVT_SHARE_DOWNLOAD_IMAGE,
                                    &[(GA4_PARAM_SHARE_URL, u.as_str())],
                                );
                            }
                            None => analytics::track_event(GA4_EVT_SHARE_DOWNLOAD_IMAGE, &[]),
                        }
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

                ui.add_space(6.0);
                if ui.button("Open").clicked() {
                    analytics::track_event(GA4_EVT_OPEN_PRESET_PICKER, &[]);
                    next_app_screen.set(AppScreen::PresetPicker);
                }
            },
        );
    });
}
