//! ワークスペース上のフラクタル状態に対するルール（予算に沿った深度など）。
//!
//! [`crate::core::budget::max_depth_for_budget`] と [`FractalState`] をずらさないため、
//! URL 復号・プリセット適用・UI のトグル後などはこのモジュールで深度を正規化する。

use crate::app::session::FractalState;
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
