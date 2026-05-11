//! 全面プリセット一覧。[`AppScreen::PresetPicker`](crate::app::mode_state::AppScreen::PresetPicker) のときのみ描画される。

mod header;
pub(crate) mod screen;
pub mod thumbnails;
mod tiles;

/// タイル列・上部ブランド・閉じる行・スクロール内コンテンツで使う左右対称の余白。
pub(super) const PRESET_PANEL_SIDE_MARGIN: f32 = 32.0;
/// Close 行フレームとスクロール内コンテンツの上下インナー。**Close 直下**の空きもこれと同じにする。
pub(super) const PRESET_PANEL_VERTICAL_INNER_MARGIN: f32 = 8.0;

use bevy::prelude::{Resource, ResMut};

use crate::app::session::{PlacementDrag, PlacementState};
use crate::ui::canvas::seed::DrawState;

pub use thumbnails::{PresetPickerNeedsInitialFocus, PresetThumbnailCache};

pub(crate) use screen::paint_preset_picker_screen;

/// 視覚的に強調しているタイルのインデックス。先頭 0 が「新規」。
#[derive(Resource, Debug, Default)]
pub struct PresetPickerTileSelection(pub usize);

/// `PresetPicker` 入場時に選択インデックスとフォーカス要求をやり直す。
///
/// 進行中のシード描画・配置ドラッグは捨てる。全面 UI へ遷移したフレーム以降、
/// キャンバス入力システムがスキップされると操作が中途半端に残るのを防ぐ。
pub(crate) fn reset_preset_picker_on_enter(
    mut tiles: ResMut<PresetPickerTileSelection>,
    mut focus: ResMut<PresetPickerNeedsInitialFocus>,
    mut draw_state: ResMut<DrawState>,
    mut placement: ResMut<PlacementState>,
) {
    tiles.0 = 0;
    focus.0 = true;
    match *draw_state {
        DrawState::DrawingLine { .. }
        | DrawState::MovingLine { .. }
        | DrawState::MovingEndpoint { .. } => *draw_state = DrawState::Idle,
        _ => {}
    }
    placement.drag = PlacementDrag::Idle;
}
