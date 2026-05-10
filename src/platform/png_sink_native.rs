//! （ネイティブ専用）[`crate::ports::png_export::PngExportSink`] の実装。`rfd` でユーザーに保存場所を選ばせ、そのパスへ PNG を書き込む。

#![cfg(not(target_arch = "wasm32"))]

use crate::ports::png_export::{AsyncMessageSlot, PngExportSink};

/// デスクトップ向け [`PngExportSink`]。`rfd` の保存ダイアログでユーザーが選んだパスへそのまま書き込む。
#[derive(Clone, Copy, Default)]
pub(super) struct NativePngSink;

/// キャンセル時はメッセージを触らず、確定時だけ `immediate_message` を更新する。非同期スロットは使わない。
impl PngExportSink for NativePngSink {
    /// 「名前を付けて保存」でパスを選ばせ、そのパスに `png_bytes` を書く。ユーザーがキャンセルしたら何もしない。
    ///
    /// # パラメータ
    ///
    /// * `share_sheet_text` / `wasm_async_feedback`: ネイティブでは未使用。トレイト共通シグネチャのため参照しない。
    /// * それ以外は [`PngExportSink::offer_png`](crate::ports::png_export::PngExportSink::offer_png) と同様。
    ///
    /// # 戻り値
    ///
    /// なし。保存成功または `std::io` エラー時に `immediate_message` を更新する。
    fn offer_png(
        &self,
        png_bytes: &[u8],
        suggested_filename: &str,
        _share_sheet_text: Option<&str>,
        immediate_message: &mut Option<String>,
        _wasm_async_feedback: AsyncMessageSlot,
    ) {
        match rfd::FileDialog::new()
            .set_file_name(suggested_filename)
            .save_file()
        {
            Some(path) => match std::fs::write(&path, png_bytes) {
                Ok(()) => {
                    *immediate_message = Some("Image saved".to_string());
                }
                Err(e) => {
                    *immediate_message = Some(format!("Save failed: {e}"));
                }
            },
            None => {}
        }
    }
}
