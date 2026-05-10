//! Result フラクタル用に用意した PNG バイト列を、環境ごとの「ユーザーに渡す経路」に差し込むためのポート定義だけを置く。
//!
//! ## このファイルがするもの
//! - [`PngExportSink`] トレイト（保存ダイアログ・Web Share・ダウンロード等への **入口だけ**）。
//! - 非同期でトーストを返すときの [`AsyncMessageSlot`] 型別名。
//!
//! ## しないこと
//! - PNG のレンダキャプチャ／エンコード。**メニューやパイプライン側でバイト列が揃っている前提**。
//! - ダイアログや `navigator` を直接呼ぶこと（実装は [`crate::platform`]）。

use std::sync::{Arc, Mutex};

/// `navigator.share` の完了など、`DeferredToast` の `pending` に相当する宛先。
pub type AsyncMessageSlot = Arc<Mutex<Option<String>>>;

/// [`AsyncMessageSlot`] と `immediate_message` の二系統があるのは、`navigator.share` のように完了が次フレーム以降になる経路があるため。
pub trait PngExportSink: Send + Sync {
    /// 準備済み PNG を環境固有の経路でユーザーに渡す。UI メッセージは `immediate_message`／`wasm_async_feedback` に書く。
    ///
    /// # パラメータ
    ///
    /// * `png_bytes`: エンコード済み PNG のバイト列（空でないことを呼び出し側が保証）。
    /// * `suggested_filename`: ブラウザの保存名・共有ファイル名の候補（拡張子 `.png` を含める想定）。
    /// * `share_sheet_text`: Web Share で `text` フィールドへ渡す本文。`Some("")` と同等として空は無視する実装が多い。
    /// * `immediate_message`: このメソッドの **return と同じタイミング** でユーザーに見せられる結果を代入する場所。
    ///   成功・失敗の短文（例:「Image saved」「Save failed: …」）。未使用なら `None` のままでよい。
    /// * `wasm_async_feedback`: Web Share の Promise などに成功メッセージを後から載せる `Mutex`。共有シート経路のみ使用。
    ///
    /// # 戻り値
    ///
    /// なし。エラー伝搬はしない。ユーザー向け結果はすべて `immediate_message` または別スレッド／イベントループでの `wasm_async_feedback` で表現する。
    fn offer_png(
        &self,
        png_bytes: &[u8],
        suggested_filename: &str,
        share_sheet_text: Option<&str>,
        immediate_message: &mut Option<String>,
        wasm_async_feedback: AsyncMessageSlot,
    );
}
