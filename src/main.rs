//! Fractalium のバイナリエントリ。
//!
//! クレート直下のモジュールを宣言し、ターゲット別の起動入口から `bootstrap` へ委譲する。
//! アプリ本体・UI・キャンバスは `app` / `ui` 配下に集約し、直下はエントリとインフラ境界のみとする。

mod app;
mod bootstrap;
mod core;
mod encoding;
mod fractal_presets;
mod platform;
mod ports;
mod ui;

/// ネイティブ／WASM 共通のプロセスエントリ。`App` の組み立ては `bootstrap::run` に任せる。
fn main() {
    bootstrap::run();
}
