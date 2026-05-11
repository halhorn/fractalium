//! Fractalium をライブラリとして公開する入り口。
//!
//! 実行バイナリは [`crate::bootstrap`] を起動するのみ。例や将来の統合テストがクレートの型を再利用するために置く。

pub mod app;
pub mod bootstrap;
pub mod core;
pub mod encoding;
pub mod fractal_presets;
pub mod platform;
pub mod ports;
pub mod ui;
