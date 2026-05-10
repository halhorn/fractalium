//! 共有リンク用の **URL とフラグメント（`#` 以降）** を環境によって読み書きするためのポート定義。
//!
//! 「現在のフラクタルを文字列へ符号化／逆に復元する」ロジックは [`crate::share`] が担う。ここにあるのは
//! **`location.href`／`history` などブラウザ OS 側 API の差し込み穴**のみ。
//!
//! ## しないこと
//! - フラクタル状態やクエリ文字列そのものの意味付け。**`fragment_body` は既に共有形式として成立しているバイト／文字である前提**。
//! - 実装細部（desktop はスタブ）。[`crate::platform`] の具象を見ること。

/// 可読フラグメント（`v=…&…`）とアドレスバーの同期。**符号化本体**は [`crate::share`]。
///
/// メソッドの一部は WASM 専用の経路からしか呼ばれず、ネイティブビルドでは静的解析上「未使用」に見える。
#[allow(dead_code)]
pub trait ShareNavigationPort: Send + Sync {
    /// 「Copy link」用のページ全体 URL（`#` と `fragment_body` を含む）。
    fn full_share_page_url_with_readable_body(&self, fragment_body: &str)
    -> Result<String, String>;

    /// `#` を `fragment_body` に合わせる（ブラウザでは `history.replaceState`）。
    fn replace_readable_fragment_body(&self, fragment_body: &str) -> Result<(), String>;

    /// 現在の `#` 本文（`#` 自身は含まない）。なければ `None`。
    fn current_fragment_body(&self) -> Option<String>;

    /// 既に同じ可読本文が表示されていれば、`replace_readable_fragment_body` を省略できる。
    fn readable_share_equals_current_fragment(&self, readable_body: &str) -> bool;
}
