//! Bevy や UI に依存しない、フラクタル向けの型と純関数をまとめる。
//!
//! 幾何の定義や深さの上限といったルールをアプリ層に混在させると、実装が散らばり、意図の追跡や単体テストが難しくなる。
//! `shape`（基図形と相似変換）、`budget`（再帰深さの予算）、`grid_snap`（編集用の格子スナップ）、`seed_preset`（基図形のプリセット）
//! に分け、それぞれの責務に含まれるデータと計算だけを置く。フラクタルとして何が起きるかを把握するときの入口にする。

pub mod budget;
pub mod fractal_line_walk;
pub mod grid_snap;
pub mod seed_preset;
pub mod shape;
