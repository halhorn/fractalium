//! ワークスペース上のフラクタル状態に対するルール（予算に沿った深度など）。
//!
//! [`crate::core::budget::max_depth_for_budget`] と [`FractalState`] をずらさないため、
//! URL 復号・プリセット適用・UI のトグル後などはこのモジュールで深度を正規化する。

use crate::core::budget::max_depth_for_budget;
use crate::state::FractalState;

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

/// フルプリセット適用など、状態ごと差し替えるときにグリッドスナップだけ維持し、深度を予算に合わせる。
///
/// `new_state` に含まれる `snap_grid` は捨て、現在の `state.snap_grid` を引き継ぐ。
///
/// # 引数
/// - `state` — 差し替え先（上書きされる）。
/// - `new_state` — プリセットなどから組み立てた次の全体状態。
///
/// # 戻り値
/// なし。
pub fn replace_fractal_state_keep_snap(state: &mut FractalState, mut new_state: FractalState) {
    let snap = state.snap_grid;
    new_state.snap_grid = snap;
    *state = new_state;
    clamp_fractal_state_depth(state);
}
