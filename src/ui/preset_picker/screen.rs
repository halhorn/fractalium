//! プリセット選択の 1 画面分（固定の Close 行、その下でブランド＋格子をスクロール）。

use bevy::prelude::NextState;
use bevy_egui::egui::{self, emath::GuiRounding as _};

use crate::app::mode_state::AppScreen;
use crate::app::session::{FractalState, PendingResultCameraFit, PlacementState, UndoStack};
use crate::app::session_rules::apply_whole_fractal_preset;
use crate::ui::canvas::seed::DrawState;

use super::header::preset_picker_brand_strip;
use super::thumbnails::{PresetPickerNeedsInitialFocus, PresetThumbnailCache};
use super::tiles::paint_preset_tile_grid;
use super::{
    PresetPickerTileSelection, PRESET_PANEL_SIDE_MARGIN, PRESET_PANEL_VERTICAL_INNER_MARGIN,
};

/// 閉じる行・直下のインナー余白（上等し）のみを概算し、ブランド〜タイルまでをスクロール領域に載せるための縦確保。
const PRESET_HEADER_RESERVE_H: f32 = 60.0 + PRESET_PANEL_VERTICAL_INNER_MARGIN;

/// プリセット選択 UI をウィンドウ全面に載せ、[AppScreen::Editing](crate::app::mode_state::AppScreen::Editing) と排他となる。
///
/// # 引数
/// - `ctx` — egui コンテキスト。
/// - `win_w`, `win_h` — 論理ウィンドウサイズ。幅は列数の上限、高さはスクロール領域の最大高さに使う。
/// - `next` — 閉じる／確定でメイン編集への遷移。
/// - `needs_focus` — 先頭セルへ初回フォーカスを載せたいフラグ。
pub(crate) fn paint_preset_picker_screen(
    ctx: &egui::Context,
    win_w: f32,
    win_h: f32,
    fractal: &mut FractalState,
    undo: &mut UndoStack,
    draw: &mut DrawState,
    placement: &mut PlacementState,
    pending_fit: &mut PendingResultCameraFit,
    next: &mut NextState<AppScreen>,
    thumbnails: &mut PresetThumbnailCache,
    picker_sel: &mut PresetPickerTileSelection,
    needs_focus: &mut PresetPickerNeedsInitialFocus,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical(|ui| {
            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(
                    PRESET_PANEL_SIDE_MARGIN as i8,
                    PRESET_PANEL_VERTICAL_INNER_MARGIN as i8,
                ))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                next.set(AppScreen::Editing);
                            }
                        });
                    });
                    ui.add_space(PRESET_PANEL_VERTICAL_INNER_MARGIN);
                });
            // [`ScrollArea`] 自体はウィンド横幅いっぱい。見た目の余白はスクロール内の [`Frame`] だけが担う。
            // 無限高のままだと縦スクロールが発生せず、複数行タイルの下が窓外に切れる。Close 行とその下（上端インナーと同値）の概算を除いた残り高でビューポートを固定する。
            let scroll_h = (win_h - PRESET_HEADER_RESERVE_H).max(120.0);
            let scroll_h = scroll_h.min(ui.available_height().max(1.0));
            egui::ScrollArea::vertical()
                .max_height(scroll_h)
                .auto_shrink([false, false])
                .hscroll(false)
                .show(ui, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(
                            PRESET_PANEL_SIDE_MARGIN as i8,
                            PRESET_PANEL_VERTICAL_INNER_MARGIN as i8,
                        ))
                        .show(ui, |ui| {
                            // [`Ui::available_width`](egui::Ui::available_width) だけでは横方向が膨らむ場合がある。Bevy のウィンドウ幅（外周マージン込み）との小さい方で列数と幅を抑える。
                            let from_bevy = (win_w - 2.0 * PRESET_PANEL_SIDE_MARGIN).max(120.0);
                            let from_egui = ui.available_width().min(ui.max_rect().width());
                            // 親から渡す折り返し幅を論理グリッドに載せ、`paint_preset_tile_grid` と同じ規則にする。
                            let preset_grid_wrap_w = from_egui
                                .min(from_bevy)
                                .max(1.0)
                                .round_ui();
                            preset_picker_brand_strip(ui);
                            if let Some(choice) = paint_preset_tile_grid(
                                ui,
                                preset_grid_wrap_w,
                                thumbnails,
                                &mut picker_sel.0,
                                &mut needs_focus.0,
                            ) {
                                apply_whole_fractal_preset(
                                    choice,
                                    fractal,
                                    undo,
                                    draw,
                                    placement,
                                    pending_fit,
                                );
                                next.set(AppScreen::Editing);
                            }
                        });
                });
        });
    });
}
