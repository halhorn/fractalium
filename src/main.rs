//! Fractalium のバイナリエントリ。
//!
//! クレート直下のモジュールを宣言し、ターゲット別の起動入口から `bootstrap` へ委譲する。

mod app;
mod bootstrap;
mod core;
mod edit;
mod encoding;
mod fractal;
mod fractal_presets;
mod grid;
mod placement;
mod platform;
mod platform_handles;
mod ports;
mod result_export;
mod seed_shape;
mod share;
mod state;
mod toast;
mod ui;
mod view;

pub use bootstrap::{
    EditCamera, PlacementCamera, ResultCamera, edit_layer, placement_layer, result_export_layer,
    result_layer,
};

/// ネイティブ／WASM 共通のプロセスエントリ。`App` の組み立ては `bootstrap::run` に任せる。
fn main() {
    bootstrap::run();
}
