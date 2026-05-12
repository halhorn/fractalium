//! Result ビュー上の Depth スライダーと Show generations（レイアウトの貫通矩形を更新する UI）。

use bevy::prelude::Vec2;
use bevy_egui::egui;

use crate::analytics;
use crate::app::session::{CanvasLayout, FractalState, ScreenRect};
use crate::app::session_rules::clamp_fractal_state_depth;
use crate::core::budget::max_depth_for_budget;

/// Depth スライダー右隣のドラッグ値・「Show generations」ボタン分を差し引いたスライダー最大幅（論理 px）。
const DEPTH_SLIDER_RESERVE_OTHER: f32 = 284.0;

/// GA4 カスタムイベント名（Show generations の切り替え）。
const GA4_EVT_SHOW_GENERATIONS_TOGGLE: &str = "fractalium_show_generations_toggle";
/// GA4 イベントパラメータキー（`0` / `1` = オフ／オン）。
const GA4_PARAM_ENABLED: &str = "enabled";

/// Depth / Show generations のオーバーレイを描画し、論理ピクセル矩形を `layout` に書き込む（ワールド入力の貫通防止用）。
///
/// `result_rect` が小さすぎる場合は `layout.result_depth_controls_rect` を None にする。
pub(crate) fn paint_depth_controls(
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
その予算を超える深さは選べません。",
                        );

                    let depth_cap = max_depth_for_budget(
                        state.base_shape.lines.len(),
                        state.replicas.len(),
                        state.show_all_generations,
                    );
                    let range = 1..=depth_cap;

                    let h = ui.spacing().interact_size.y;
                    let slider_w = (result_rect.width() - pad.x * 2.0 - DEPTH_SLIDER_RESERVE_OTHER).clamp(80.0, 220.0);

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
                        analytics::track_event(
                            GA4_EVT_SHOW_GENERATIONS_TOGGLE,
                            &[(
                                GA4_PARAM_ENABLED,
                                if state.show_all_generations { "1" } else { "0" },
                            )],
                        );
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
