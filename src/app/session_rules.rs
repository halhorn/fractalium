//! 編集中フラクタル状態に対するルール（予算に沿った深度など）。
//!
//! [`crate::core::budget::max_depth_for_budget`] と [`FractalState`] をずらさないため、
//! URL 復号・プリセット適用・UI のトグル後などはこのモジュールで深度を正規化する。

use crate::app::session::{
    FractalState, PendingResultCameraFit, PlacementDrag, PlacementState, UndoStack,
};
use crate::fractal_presets::FractalPreset;
use crate::ui::canvas::seed::DrawState;

use crate::core::budget::max_depth_for_budget;

/// 現在の基図形・複製・描画モードから見た予算で `depth` を [1, cap] に収める。
///
/// # 引数
/// - `state` — `depth` のほか `base_shape.lines`・`replicas`・`show_all_generations` を読む。
///
/// # 戻り値
/// なし。`state.depth` のみ更新する。
pub fn clamp_fractal_state_depth(state: &mut FractalState) {
    let cap = max_depth_for_budget(
        state.base_shape.lines.len(),
        state.replicas.len(),
        state.show_all_generations,
    );
    state.depth = state.depth.min(cap).max(1);
}

/// フラクタル定義だけを別状態へ載せ替え、深さを予算へ合わせる。
///
/// プリセット適用など、幾何・レプリカ一式を丸ごと入れ替える経路で使う。
///
/// # 引数
/// - `state` — 差し替え先（上書きされる）。
/// - `new_state` — プリセットなどから組み立てた次のフラクタル定義。
///
/// # 戻り値
/// なし。
pub fn replace_fractal_state(state: &mut FractalState, new_state: FractalState) {
    *state = new_state;
    clamp_fractal_state_depth(state);
}

/// 全体プリセット（またはエディタ相当の「新規／空」）を一度に適用し、既存プリセット項目と同一の副作用に揃える。
///
/// # 引数
/// - `choice` — `Some(p)` で該当プリセット、`None` で [`FractalState::default`]。
/// - `fractal` — 上書き対象。
/// - `undo` — 直前状態を積む。
/// - `draw` — シード編集をアイドルへ戻す。
/// - `placement` — 選択とドラッグをリセットする（リンク貼り付けバッファは既存プリセット項目と同様に変更しない）。
/// - `pending_result_fit` — 結果キャンバスをフラクタル編集ビューに合わせて再フィットする。
///
/// # 戻り値
/// なし。
pub fn apply_whole_fractal_preset(
    choice: Option<FractalPreset>,
    fractal: &mut FractalState,
    undo: &mut UndoStack,
    draw: &mut DrawState,
    placement: &mut PlacementState,
    pending_result_fit: &mut PendingResultCameraFit,
) {
    undo.push(fractal.clone());
    match choice {
        Some(p) => replace_fractal_state(fractal, p.build()),
        None => replace_fractal_state(fractal, FractalState::default()),
    }
    *draw = DrawState::Idle;
    placement.selected = None;
    placement.drag = PlacementDrag::Idle;
    pending_result_fit.0 = true;
}
