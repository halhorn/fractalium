//! [`crate::ports`] にある trait に対して、OS／ブラウザ向けの **実装オブジェクトだけ** を差し込むモジュール。
//!
//! [`share_navigation_arc`] と [`png_export_sink_arc`] が返すものを、そのまま
//! [`crate::share::ShareNavigation`] と [`crate::result_export::ResultImageOutlet`] に載せて使う。
//!
//! ## しないこと
//! - フラクタル・Undo・共有クエリ符号化。**ワークスペース状態は読まず**、渡されたパラメータを環境へ送るだけ。

#[cfg(not(target_arch = "wasm32"))]
mod png_sink_native;
#[cfg(target_arch = "wasm32")]
mod png_sink_wasm;
#[cfg(not(target_arch = "wasm32"))]
mod share_nav_native;
#[cfg(target_arch = "wasm32")]
mod share_nav_wasm;

use std::sync::Arc;

use crate::ports::png_export::PngExportSink;
use crate::ports::share_link::ShareNavigationPort;

/// 起動時に [`crate::share::ShareNavigation`] へ載せる [`ShareNavigationPort`]。
#[cfg(not(target_arch = "wasm32"))]
pub fn share_navigation_arc() -> Arc<dyn ShareNavigationPort + Send + Sync> {
    Arc::new(share_nav_native::DesktopShareNavigator)
}

#[cfg(target_arch = "wasm32")]
pub fn share_navigation_arc() -> Arc<dyn ShareNavigationPort + Send + Sync> {
    Arc::new(share_nav_wasm::WasmShareNavigator)
}

/// 起動時に [`crate::result_export::ResultImageOutlet`] へ載せる [`PngExportSink`]。
#[cfg(not(target_arch = "wasm32"))]
pub fn png_export_sink_arc() -> Arc<dyn PngExportSink + Send + Sync> {
    Arc::new(png_sink_native::NativePngSink)
}

#[cfg(target_arch = "wasm32")]
pub fn png_export_sink_arc() -> Arc<dyn PngExportSink + Send + Sync> {
    Arc::new(png_sink_wasm::WasmPngSink)
}
