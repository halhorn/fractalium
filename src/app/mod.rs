//! アプリケーション層のユースケースと共有形式・深度などワークスペース上のルール。
//!
//! `ui` はここを経由して状態を整合させ、共有やプリセットの詳細ロジックはプレゼンテーションだけに閉じない。
//!
//! - [`share::payload`] … `#v=…` フラグメントのドメイン意味・検証。クエリ形は [`crate::encoding::flat_query_codec`]。
//! - [`session_rules`] … 再帰予算に沿った深度クランプと、プリセット適用時の状態の載せ替え。

pub mod export;
pub mod platform_handles;
pub mod session;
pub mod session_rules;
pub mod share;
