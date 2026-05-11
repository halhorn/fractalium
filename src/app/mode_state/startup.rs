//! 起動 1 回だけの `AppScreen` 決定。`bootstrap` と WASM の URL ハイドレーションの順序前提をコメントとシステム順で明示する。
//!
//! - **ネイティブ**: [`BootstrapUsedPresetEnv`] が真なら [`AppScreen::Editing`](super::routing::AppScreen::Editing)。
//! - **WASM**: [`ShareUrlRestoredFractal`] が真なら `Editing`。それ以外は [`AppScreen::PresetPicker`](super::routing::AppScreen::PresetPicker)。

use bevy::prelude::{NextState, Res, ResMut, Resource};

use super::routing::AppScreen;

/// `FRACTALIUM_BOOT_PRESET` が有効で初期 [`FractalState`](crate::app::session::FractalState) がそれ由来のとき真。
///
/// `bootstrap::run` が `initial_fractal_state` の結果とともに書き込む。
/// WASM は [`AppScreen`] 判定には使わず、ネイティブ専用（このフィールドは wasm ビルドで未読になりうる）。
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct BootstrapUsedPresetEnv(pub bool);

/// WASM で URL から復元できた場合に真。ネイティブビルドではフラグのみ保持し参照しない。
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct ShareUrlRestoredFractal(pub bool);

/// WASM では [`crate::app::share::sync::hydrate_from_url`] より後に登録済みとして、`NextState` を積む。
///
/// # 引数
/// - `next` — 遷移キュー。
/// - `boot` — ネイティブ向けブートプリセットの有無。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve_initial_app_screen(
    mut next: ResMut<NextState<AppScreen>>,
    boot: Res<BootstrapUsedPresetEnv>,
) {
    if boot.0 {
        next.set(AppScreen::Editing);
    }
}

/// # 引数
/// - `next` — 遷移キュー。
/// - `url` — URL 復元の有無（ブート環境変数はネイティブ専用のためここでは見ない）。
#[cfg(target_arch = "wasm32")]
pub(crate) fn resolve_initial_app_screen(
    mut next: ResMut<NextState<AppScreen>>,
    url: Res<ShareUrlRestoredFractal>,
) {
    if url.0 {
        next.set(AppScreen::Editing);
    }
}
