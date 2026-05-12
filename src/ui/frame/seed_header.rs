//! Seed（種図形）ブロックのヘッダ: プリセットメニュー・線の削除・全消去。

use bevy_egui::egui;

use crate::analytics;
use crate::app::session::{FractalState, UndoStack};
use crate::core::seed_preset::BaseShapePreset;
use crate::ui::canvas::seed::DrawState;

use super::chrome::step_glyph_button;

/// GA4 カスタムイベント名（Seed の Shape メニューで基図形プリセットを選んだとき）。
const GA4_EVT_SEED_SHAPE_PICK: &str = "fractalium_seed_shape_pick";
/// GA4 カスタムイベント名（Seed で選択中の線を削除）。
const GA4_EVT_SEED_DELETE_LINE: &str = "fractalium_seed_delete_line";
/// GA4 カスタムイベント名（Seed の線をすべて消去）。
const GA4_EVT_SEED_CLEAR: &str = "fractalium_seed_clear";
/// GA4 イベントパラメータキー（選んだ Shape の表示名）。
const GA4_PARAM_SHAPE_NAME: &str = "shape_name";

/// Seed ブロックのヘッダ右側: **Shape / - / Clear**（ワイド・ナロー共通）。
///
/// `minus_compact` が true のとき `-` は [`step_glyph_button`] サイズ（ナロー用）。
pub(crate) fn base_shape_header_buttons(
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
                analytics::track_event(
                    GA4_EVT_SEED_SHAPE_PICK,
                    &[(GA4_PARAM_SHAPE_NAME, preset.label())],
                );
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
            analytics::track_event(GA4_EVT_SEED_DELETE_LINE, &[]);
        }
    } else if ui
        .add_enabled(can_del, egui::Button::new("-").small())
        .clicked()
        && let DrawState::Selected(idx) = *draw_state
    {
        undo_stack.push(state.clone());
        state.base_shape.lines.remove(idx);
        *draw_state = DrawState::Idle;
        analytics::track_event(GA4_EVT_SEED_DELETE_LINE, &[]);
    }

    if ui.small_button("Clear").clicked() {
        undo_stack.push(state.clone());
        state.base_shape.lines.clear();
        *draw_state = DrawState::Idle;
        analytics::track_event(GA4_EVT_SEED_CLEAR, &[]);
    }
}
