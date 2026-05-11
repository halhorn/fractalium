//! プリセット画面のブランドブロック（正式ロゴ差し替え前のプレースホルダ）。多くは [`ScrollArea`](egui::ScrollArea) の先頭で使う。

use bevy_egui::egui;

/// スクロール内先頭に置く識別テキストと下罫の余白。
///
/// 端からのインナー余白は `screen.rs` のスクロール内 [`Frame`](egui::Frame) が担う。
///
/// # 引数
/// - `ui` — 親の [`egui::Ui`]。
pub fn preset_picker_brand_strip(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("Fractalium")
                .strong()
                .size(28.0)
                .color(egui::Color32::from_rgb(220, 225, 235)),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Choose a starting fractal")
                .small()
                .color(egui::Color32::from_rgb(140, 145, 160)),
        );
        ui.add_space(12.0);
    });
    ui.separator();
}
