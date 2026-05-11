//! 角丸カードの格子（行分割レイアウト）。先頭は「新規」、続けて [`FractalPreset::ALL`](crate::fractal_presets::FractalPreset::ALL)。

use bevy_egui::egui::{self, emath::GuiRounding as _};

use crate::fractal_presets::FractalPreset;

use super::thumbnails::PresetThumbnailCache;

/// タイル間の余白・基準（フルサイズ時の論理値）。
const GRID_GAP_BASE: f32 = 16.0;
/// フルサイズ時のカード外枠論理横幅（画像＋キャプション＋インナーマージン込み）。
const TILE_OUTER_W_BASE: f32 = 196.0;
/// フルサイズ時のサムネ論理サイズ。
const THUMB_W_BASE: f32 = 158.0;
/// フルサイズ時のサムネ論理高さ。
const THUMB_H_BASE: f32 = 122.0;
/// フルサイズ時のカード [`Frame`] 内マージン（各辺）。
const TILE_INNER_MARGIN_BASE: f32 = 12.0;
/// コンテナ横幅がこれ以上なら縮小せずベース寸法どおり並べる。
const TILE_LAYOUT_FULL_WIDTH_PT: f32 = 540.0;
/// コンテナ横幅がこれを下回ると縮小係数 [`TILE_LAYOUT_MIN_SCALE`] で張り付く。
const TILE_LAYOUT_NARROW_FLOOR_PT: f32 = 300.0;
/// 狭い画面での論理サイズ縮小の下限係数。
const TILE_LAYOUT_MIN_SCALE: f32 = 0.64;

/// narrow コンテナ幅に応じたタイル一式の論理寸法。列分割と各カード描画で共有する。
#[derive(Clone, Copy)]
struct PresetTileLayout {
    /// 行・列の [`Ui::spacing`](egui::Ui::spacing) に使うギャップ。
    gap: f32,
    /// 1 列あたりの横方向ストライド（カード外幅＋右隣の `gap`）。
    stride_x: f32,
    /// カード外枠の論理幅（`show` クロージャ内での [`Ui::set_width`](egui::Ui::set_width)）。
    outer_w: f32,
    /// サムネの論理幅。
    thumb_w: f32,
    /// サムネの論理高さ。
    thumb_h: f32,
    /// カード内マージン（四辺同値、[`Margin::symmetric`](egui::Margin::symmetric)）。
    inner_margin: i8,
    /// キャプションのフォント論理サイズ。
    caption_pt: f32,
    /// サムネ直上・キャプション下の小さな縦すき間。
    pad_small: f32,
    /// サムネとキャプションのあいだ。
    pad_medium: f32,
    /// カード外枠の角丸。
    corner_outer: egui::CornerRadius,
    /// テクスチャ未取得プレースホルダ矩形の角丸。
    corner_thumb_placeholder: egui::CornerRadius,
}

/// コンテナ論理横幅からタイルの寸法・ストライドへまとめる。
///
/// # 引数
/// `wrap_pt` — 親で測った折り返し幅（コンテナ論理横幅）。
///
/// # 戻り値
/// 列数算出と [`paint_one_tile`] にそのまま渡せるセット。
fn preset_tile_layout_for_wrap_pt(wrap_pt: f32) -> PresetTileLayout {
    let wrap_pt = sanitize_grid_wrap_width(wrap_pt);
    let scale = preset_tile_layout_scale(wrap_pt);
    // 論理座標を emath の `round_ui`（`GUI_ROUNDING` = 1/32 pt 刻み）に揃えると、
    // デバッグの「Unaligned」警告と累積誤差が出にくい。
    let outer_w = (TILE_OUTER_W_BASE * scale).round_ui();
    let thumb_w = (THUMB_W_BASE * scale).round_ui();
    let thumb_h = (THUMB_H_BASE * scale).round_ui();
    let gap = (GRID_GAP_BASE * scale).max(10.0).round_ui();
    let stride_x = (outer_w + gap).round_ui();

    PresetTileLayout {
        gap,
        stride_x,
        outer_w,
        thumb_w,
        thumb_h,
        inner_margin: (TILE_INNER_MARGIN_BASE * scale)
            .max(6.0)
            .round_ui()
            .round()
            .clamp(6.0, i8::MAX as f32) as i8,
        caption_pt: ((14.5 * scale).max(11.0)).round_ui(),
        pad_small: ((4.0 * scale).max(2.0)).round_ui(),
        pad_medium: ((6.0 * scale).max(3.0)).round_ui(),
        corner_outer: egui::CornerRadius::from((14.0 * scale).round_ui()),
        corner_thumb_placeholder: egui::CornerRadius::from((6.0 * scale).round_ui()),
    }
}

/// コンテナ幅に応じたタイル論理サイズ係数。
///
/// # 引数
/// `wrap_pt` — サニタイズ済みまたは未検査の折り返し幅。[`sanitize_grid_wrap_width`] 済みを想定。
///
/// # 戻り値
/// `1.0`（ゆとり幅）〜 [`TILE_LAYOUT_MIN_SCALE`]（極細）のクランプ係数。
fn preset_tile_layout_scale(wrap_pt: f32) -> f32 {
    if wrap_pt >= TILE_LAYOUT_FULL_WIDTH_PT {
        return 1.0;
    }
    if wrap_pt <= TILE_LAYOUT_NARROW_FLOOR_PT {
        return TILE_LAYOUT_MIN_SCALE;
    }
    let band = TILE_LAYOUT_FULL_WIDTH_PT - TILE_LAYOUT_NARROW_FLOOR_PT;
    let t = ((wrap_pt - TILE_LAYOUT_NARROW_FLOOR_PT) / band).clamp(0.0, 1.0);
    TILE_LAYOUT_MIN_SCALE + t * (1.0 - TILE_LAYOUT_MIN_SCALE)
}

/// [`ScrollArea`](egui::ScrollArea) より上の親で測った利用可能横幅を、列数の算出に使う。
///
/// 数値が壊れているときだけフォールバックする（通常は `screen` で `available_width()` を渡す）。
fn sanitize_grid_wrap_width(measured_available_width: f32) -> f32 {
    let w = if measured_available_width.is_finite() && measured_available_width > 0.0 {
        measured_available_width
    } else {
        480.0
    };
    w.round_ui()
}

/// `wrap_w` に収まる列数（最低 1）。[`PresetTileLayout::stride_x`] で幅を明示する。
fn column_count_for_layout(wrap_w: f32, layout: &PresetTileLayout) -> usize {
    if !(layout.stride_x.is_finite()) || layout.stride_x <= 0.0 {
        return 1;
    }
    let n = ((wrap_w + layout.gap) / layout.stride_x).floor() as usize;
    n.max(1)
}

/// 角丸タイルを行に分割して配置し、クリック時に返す。
///
/// # 引数
/// - `ui` — 親 UI（多くは [`ScrollArea`](egui::ScrollArea) 直下）。入った直後に [`Ui::set_max_width`] で幅の上限を親と揃える。
/// - `grid_wrap_width` — `ScrollArea` の外で測った [`Ui::available_width`]。列数と `set_max_width` に使う。
/// - `thumbnails` — 遅延ロード済みテクスチャ。
/// - `selected_index` — 視覚選択のインデックス（先頭が新規）。
/// - `needs_initial_focus` — 画面入場直後だけ真にし、先頭タイルへフォーカスを渡す。
///
/// # 戻り値
/// そのフレームに確定した選択。
pub fn paint_preset_tile_grid(
    ui: &mut egui::Ui,
    grid_wrap_width: f32,
    thumbnails: &mut PresetThumbnailCache,
    selected_index: &mut usize,
    needs_initial_focus: &mut bool,
) -> Option<Option<FractalPreset>> {
    let total = 1 + FractalPreset::ALL.len();
    *selected_index = (*selected_index).min(total.saturating_sub(1));

    let mut picked: Option<Option<FractalPreset>> = None;

    let wrap_w = sanitize_grid_wrap_width(grid_wrap_width);
    let layout = preset_tile_layout_for_wrap_pt(grid_wrap_width);
    ui.set_max_width(wrap_w);

    let cols = column_count_for_layout(wrap_w, &layout);

    ui.spacing_mut().item_spacing = egui::vec2(layout.gap, layout.gap);

    let mut slots: Vec<(usize, &'static str, TileThumb)> =
        Vec::with_capacity(1 + FractalPreset::ALL.len());
    slots.push((0, "New workspace", TileThumb::New));
    for (i, &preset) in FractalPreset::ALL.iter().enumerate() {
        slots.push((i + 1, preset.label(), TileThumb::Preset(preset)));
    }

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(layout.gap, layout.gap);
        for row in slots.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(layout.gap, layout.gap);
                for &(idx, caption, thumb) in row {
                    let r = paint_one_tile(
                        ui,
                        thumbnails,
                        idx,
                        *selected_index == idx,
                        caption,
                        thumb,
                        &layout,
                    );
                    if *needs_initial_focus && idx == 0 {
                        r.request_focus();
                        *needs_initial_focus = false;
                    }
                    if r.hovered() {
                        *selected_index = idx;
                    }
                    if r.clicked() {
                        picked = match thumb {
                            TileThumb::New => Some(None),
                            TileThumb::Preset(p) => Some(Some(p)),
                        };
                    }
                }
            });
        }
    });

    picked
}

#[derive(Clone, Copy)]
enum TileThumb {
    New,
    Preset(FractalPreset),
}

fn paint_one_tile(
    ui: &mut egui::Ui,
    thumbnails: &mut PresetThumbnailCache,
    _tile_index: usize,
    selected: bool,
    caption: &str,
    thumb: TileThumb,
    layout: &PresetTileLayout,
) -> egui::Response {
    let bg = if selected {
        egui::Color32::from_rgb(52, 58, 72)
    } else {
        egui::Color32::from_rgb(36, 40, 48)
    };
    let stroke = if selected {
        egui::Stroke::new(
            1.5,
            egui::Color32::from_rgb(120, 170, 220),
        )
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_rgb(62, 66, 78))
    };

    let outer = egui::Frame::default()
        .fill(bg)
        .corner_radius(layout.corner_outer)
        .stroke(stroke)
        .inner_margin(egui::Margin::symmetric(layout.inner_margin, layout.inner_margin))
        .show(ui, |ui| {
            ui.set_width(layout.outer_w);
            ui.vertical_centered(|ui| {
                ui.add_space(layout.pad_small);

                let thumb_ctx = ui.ctx().clone();
                let tex = match thumb {
                    TileThumb::New => thumbnails.texture_new_tile(&thumb_ctx),
                    TileThumb::Preset(p) => thumbnails.texture_for_preset(&thumb_ctx, p),
                };

                if let Some(t) = tex {
                    ui.add(
                        egui::widgets::Image::new(egui::load::SizedTexture::new(
                            t.id(),
                            t.size_vec2(),
                        ))
                        .fit_to_exact_size(egui::vec2(layout.thumb_w, layout.thumb_h)),
                    );
                } else {
                    let (r, _) = ui.allocate_exact_size(
                        egui::vec2(layout.thumb_w, layout.thumb_h),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(
                        r,
                        layout.corner_thumb_placeholder,
                        egui::Color32::from_gray(44),
                    );
                }

                ui.add_space(layout.pad_medium);
                ui.label(
                    egui::RichText::new(caption)
                        .size(layout.caption_pt)
                        .color(egui::Color32::from_rgb(215, 218, 228)),
                );
                ui.add_space(layout.pad_small);
            });
        });

    outer.response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
