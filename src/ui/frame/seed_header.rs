//! Seed（種図形）ブロックのヘッダ: プリセットメニュー・線の削除・全消去。

use bevy_egui::egui;

use crate::app::session::{FractalState, UndoStack};
use crate::core::seed_preset::BaseShapePreset;
use crate::ui::canvas::seed::DrawState;

use super::chrome::step_glyph_button;

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
