//! 共有 URL と PNG 出力の **2 種ポート**をひとつの Bevy [`Resource`] にまとめる箱。
//!
//! [`ShareNavigation`] と [`ResultImageOutlet`] を **1 つの [`Resource`]** に束ね、`params_panel` などではこの型だけ参照すればよいようにする。[`crate::platform`] のファクトリを呼ぶのは `main` だけ。
use bevy::prelude::Resource;

use crate::app::export::ResultImageOutlet;
use crate::app::share::sync::ShareNavigation;

/// 起動時に `platform` のファクトリで詰めるインファ束。複数ポートをひとつの Bevy の `Resource` に載せるためのコンテナ。
#[derive(Resource, Clone)]
pub struct PlatformHandles {
    /// 共有リンク用 URL／フラグメントのプラットフォーム実装。
    pub share_navigation: ShareNavigation,
    /// Result PNG の保存／共有プラットフォーム実装。
    pub result_png_outlet: ResultImageOutlet,
}
