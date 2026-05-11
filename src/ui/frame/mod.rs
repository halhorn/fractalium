//! フレーム内の見た目: レイアウト・パラメータパネル・グローバルバー・タイトル帯など。
//!
//! egui の矩形から各 [`bevy::camera::Camera`] の [`bevy::camera::Viewport`] へ同期する処理は
//! [`crate::ui::viewport_bridge`]（本モジュールとは別責務）。

pub mod chrome;
pub mod depth_controller;
pub mod global_bar;
pub mod layout;
pub mod params;
pub mod seed_header;
