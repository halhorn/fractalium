//! タイトル帯・Seed/Placement 用キャンバス枠など、複数レイアウトで共通の視覚コンテナ。

use bevy_egui::egui;

/// ナローレイアウトの `+` / `-` 用。高さはインタラクト高さ、幅はワイド用より詰めてヘッダに収める。
///
/// `label` はボタン表示文字。このボタンの [`egui::Response`] を返す。
pub(crate) fn step_glyph_button(ui: &mut egui::Ui, label: &'static str) -> egui::Response {
    let h = ui.spacing().interact_size.y;
    let w = (h + 14.0) * 0.75;
    ui.add_sized(egui::vec2(w, h), egui::Button::new(label).small())
}

/// アプリ名を載せるトップ帯の背景・枠線スタイル。
pub(crate) fn app_title_panel_frame() -> egui::Frame {
    egui::Frame::default()
        .inner_margin(egui::Margin::symmetric(10, 8))
        .fill(egui::Color32::from_rgb(26, 26, 34))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 55, 72)))
}

/// タイトル帯の中央にアプリ名を表示する。
pub(crate) fn app_title_bar_contents(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.label(egui::RichText::new("Fractalium").heading().strong());
    });
}

/// Seed / Placement ブロックの半透明枠（中身はワールド側と同色に見えるよう透明塗り）。
pub(crate) fn canvas_block_frame() -> egui::Frame {
    egui::Frame::default()
        .inner_margin(egui::Margin::same(8))
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 55, 72)))
}

/// `title` とヘッダ操作、`header_buttons` を並べたあと、正方形に近いキャンバス予約領域の [`egui::Rect`] を返す。
pub(crate) fn show_canvas_block(
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
