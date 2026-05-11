//! **インフラ I/O とアプリ本体の間**に挟む trait とサブモジュールの一覧。
//!
//! - [`png_export`]: Result PNG をユーザーへ渡す
//! - [`share_link`]: アドレスバー／フラグメントの同期と「リンク全体」の組み立て
//!
//! 具象実装は `crate::platform`。[`crate::app::platform_handles::PlatformHandles`] を [`bevy::prelude::Resource`] に載せ、起動時に起動コードが詰める。

pub mod png_export;
pub mod share_link;
