//! 共有リンク: フラグメント本文のドメイン検証（[`payload`]）と URL 同期のオーケストレーション（[`sync`]）。

pub mod payload;
pub mod sync;

pub use sync::{ShareNavigation, SharePlugin};
