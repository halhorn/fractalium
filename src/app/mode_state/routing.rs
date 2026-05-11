//! メイン編集か全面プリセット選択かだけを表す画面上位モード。
//!
//! 編集中フラクタルの [`FractalState`](crate::app::session::FractalState) やキャンバス由来の状態とは名前を混ぜない。

use bevy::prelude::States;

/// Bevy の [`States`] として、`egui` ルートの排他パネル切り替えに使う。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Default, States)]
pub enum AppScreen {
    /// 起動直後の既定。**共有 URL が効かないとき**など、ユーザーにプリセットを選ばせる。
    #[default]
    PresetPicker,
    /// 従来の Seed / Placement / Result 付き編集 UI。
    Editing,
}
