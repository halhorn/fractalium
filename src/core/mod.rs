//! Bevy や UI に依存しない、フラクタル向けの型と純関数をまとめる。
//!
//! 幾何の定義や深さの上限といったルールをアプリ層に混在させると、実装が散らばり、意図の追跡や単体テストが難しくなる。
//! `shape`（基図形と相似変換）、`budget`（再帰深さの予算）、`grid_snap`（編集用の格子スナップ）、`seed_preset`（基図形のプリセット）
//! に分け、それぞれの責務に含まれるデータと計算だけを置く。フラクタルとして何が起きるかを把握するときの入口にする。

pub mod budget;
pub mod grid_snap;
pub mod seed_preset;
pub mod shape;

/// 再帰の `depth` として共有 URL や UI が許容する絶対上限（ネイティブと WASM で値が異なる）。
///
/// 入力検証と再帰・メッシュ生成で同じ上限を使い、スタックや GPU 負荷の暴走を防ぐために定義する。
pub use budget::FRACTAL_DEPTH_HARD_CAP;

/// 再帰の出発図形。基図形を構成する線分の集合。
pub use shape::BaseShape;
/// 正規化座標上の開線分。端点は `a` から `b`。
pub use shape::Line;
/// レプリカの一様スケールの上限。編集・共有形式の検証・UI の上限を揃える。
pub use shape::REPLICA_SCALE_MAX;
/// レプリカの一様スケールの下限。編集・共有形式の検証・UI の下限を揃える。
pub use shape::REPLICA_SCALE_MIN;
/// IFS の各枝に対応する相似変換（一様スケール・回転のあとの位置オフセットを含む）。
pub use shape::Replica;
