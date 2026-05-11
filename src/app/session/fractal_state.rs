//! 保存・共有の芯となるフラクタル定義リソース（`FractalState`）。

use bevy::prelude::*;

use crate::core::shape::{BaseShape, Replica};

/// フラクタル全体の状態を表す Bevy リソース。
/// 「基図形 → 複製ルール → 再帰深さ」の 3 要素でフラクタルが一意に決まる。
/// 座標は正規化キャンバス座標 [-1, 1] x [-1, 1] を用いる。
#[derive(Resource, Clone)]
pub struct FractalState {
    /// 再帰の元となる基図形（線分の集合）
    pub base_shape: BaseShape,
    /// 再帰時に基図形を配置する複製変換のリスト
    pub replicas: Vec<Replica>,
    /// 再帰の深さ（1 = 基図形のみ、2 以上で replicas が適用される）
    pub depth: u32,
    /// true のとき、末端世代だけでなく途中世代の図形も描画する
    pub show_all_generations: bool,
}

impl Default for FractalState {
    /// 初期状態：基図形・複製ともに空、深さは 4。
    fn default() -> Self {
        Self {
            base_shape: BaseShape::default(),
            replicas: vec![],
            depth: 4,
            show_all_generations: false,
        }
    }
}
