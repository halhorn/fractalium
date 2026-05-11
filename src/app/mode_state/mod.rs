//! アプリ画面上位モードのみを表す状態（プリセット一覧とメイン編集のどちらを描くか）。
//!
//! Bevy の [`States`](bevy::prelude::States) と [`NextState`](bevy::prelude::NextState) で遷移し、
//! [`FractalState`](crate::app::session::FractalState) とは別名前の [`AppScreen`] に集約する。

mod plugin;
mod routing;
pub(crate) mod startup;

pub use plugin::AppScreenPlugin;
pub use routing::AppScreen;
