//! egui 上に短時間表示する汎用トースト。`EguiToast::show` でキューし、`paint` で描画する。

use std::sync::{Arc, Mutex};

use bevy::prelude::Resource;
use bevy_egui::egui;

const AREA_ID: &str = "fractalium_egui_toast";

/// 不透明になるまでの時間（急峻に立ち上げる）。旧 0.1s の 1.5 倍。
const FADE_IN_SECS: f64 = 0.15;
/// フェードアウトに使う固定時間。
const FADE_OUT_SECS: f64 = 0.5;

#[derive(Clone)]
struct ToastLine {
    text: String,
    started_at: f64,
    /// `show_for` の合計時間から `FADE_IN + FADE_OUT` を引いた値（下限 0）。
    opaque_hold_secs: f64,
}

/// 同期で積むメッセージ。`navigator.share` の Promise などは内部 `pending` に届き、egui 冒頭で `flush_async_to_message` してから表示する。
#[derive(Resource, Clone)]
pub struct DeferredToast {
    pub message: Option<String>,
    pending: Arc<Mutex<Option<String>>>,
}

impl Default for DeferredToast {
    fn default() -> Self {
        Self {
            message: None,
            pending: Arc::new(Mutex::new(None)),
        }
    }
}

impl DeferredToast {
    pub fn flush_async_to_message(&mut self) {
        let Ok(mut g) = self.pending.lock() else {
            return;
        };
        if let Some(msg) = g.take() {
            self.message = Some(msg);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn async_toast_sink(&self) -> Arc<Mutex<Option<String>>> {
        self.pending.clone()
    }
}

#[derive(Resource)]
pub struct EguiToast {
    active: Option<ToastLine>,
}

impl Default for EguiToast {
    fn default() -> Self {
        Self { active: None }
    }
}

impl EguiToast {
    /// `show` で使うトースト全体の長さ（秒）。フェードイン・アウトを除いた分は完全不透明のホールド。
    pub const DEFAULT_TOTAL_SECS: f64 = 1.8;
    /// 本文。既定ボディの約 2 倍。
    pub const TEXT_SIZE: f32 = 28.0;

    /// 既定の合計時間で画面中央に出す。
    pub fn show(&mut self, ctx: &egui::Context, message: impl Into<String>) {
        self.show_for(ctx, message, Self::DEFAULT_TOTAL_SECS);
    }

    /// `total_secs`: 表示開始から完全消失までの合計。内訳は固定のフェードイン・アウトと残りホールド。
    pub fn show_for(&mut self, ctx: &egui::Context, message: impl Into<String>, total_secs: f64) {
        let now = ctx.input(|i| i.time);
        let min_total = FADE_IN_SECS + FADE_OUT_SECS;
        let total = total_secs.max(min_total);
        let opaque_hold_secs = (total - FADE_IN_SECS - FADE_OUT_SECS).max(0.0);
        self.active = Some(ToastLine {
            text: message.into(),
            started_at: now,
            opaque_hold_secs,
        });
    }

    /// `None` ならトースト終了（消去）。
    fn alpha_at(elapsed: f64, opaque_hold_secs: f64) -> Option<f32> {
        let fade_out_start = FADE_IN_SECS + opaque_hold_secs;
        let end = fade_out_start + FADE_OUT_SECS;

        if elapsed < 0.0 {
            return Some(0.0);
        }
        if elapsed >= end {
            return None;
        }

        if elapsed < FADE_IN_SECS {
            let x = (elapsed / FADE_IN_SECS) as f32;
            let a = 1.0 - (1.0 - x).powi(4);
            return Some(a.clamp(0.0, 1.0));
        }
        if elapsed < fade_out_start {
            return Some(1.0);
        }

        let u = ((elapsed - fade_out_start) / FADE_OUT_SECS) as f32;
        Some((1.0 - u).clamp(0.0, 1.0))
    }

    /// メインフレームの egui パスで 1 回呼ぶ。期限切れなら中身を消す。
    pub fn paint(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let Some(line) = self.active.as_ref() else {
            return;
        };

        let elapsed = now - line.started_at;
        let Some(alpha) = Self::alpha_at(elapsed, line.opaque_hold_secs) else {
            self.active = None;
            return;
        };

        let text = line.text.clone();
        let fill_a = (242.0 * alpha).round() as u8;
        let text_a = (255.0 * alpha).round() as u8;

        egui::Area::new(egui::Id::new(AREA_ID))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(45, 45, 55, fill_a))
                    .inner_margin(egui::Margin::symmetric(28, 20))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(text)
                                .strong()
                                .font(egui::FontId::proportional(Self::TEXT_SIZE))
                                .color(egui::Color32::from_rgba_unmultiplied(
                                    255, 255, 255, text_a,
                                )),
                        );
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_curve_end_and_peak() {
        let hold = 0.6;
        assert!(EguiToast::alpha_at(0.0, hold).unwrap() < 0.25);
        assert!((EguiToast::alpha_at(FADE_IN_SECS * 0.99, hold).unwrap() - 1.0).abs() < 0.2);
        assert!((EguiToast::alpha_at(FADE_IN_SECS + hold * 0.5, hold).unwrap() - 1.0).abs() < 1e-3);
        assert!(EguiToast::alpha_at(FADE_IN_SECS + hold + FADE_OUT_SECS, hold).is_none());
    }
}
