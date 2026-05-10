//! アプリケーション層のユースケースと共有形式・深度などワークスペース上のルール。
//!
//! `ui` はここを経由して状態を整合させ、共有やプリセットの詳細ロジックはプレゼンテーションだけに閉じない。
//!
//! - [`fractal_share`] … `#v=…` フラグメントのドメイン意味・検証。クエリ形は [`crate::encoding::flat_query_codec`]。
//! - [`workspace`] … 再帰予算に沿った深度クランプと、プリセット適用時の状態の載せ替え。

mod fractal_share;
mod workspace;

pub use fractal_share::{
    decode_readable_share_query, encode_state, share_sheet_text_for_export, MAX_DEPTH,
};
pub use workspace::{clamp_fractal_state_depth, replace_fractal_state_keep_snap};
