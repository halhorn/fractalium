//! （Web 専用）[`crate::ports::share_link::ShareNavigationPort`] の実装。[`crate::app::share::sync`] と同じ `from=share` 付与・クエリ削除のヘルパを使い、アドレスバーを共有形式へ合わせる。
//!
//! 起動時のフラグメント復元・メッシュ更新後の `replaceState` がここ経由となる。

use crate::app::share::sync as share;
use crate::ports::share_link::ShareNavigationPort;

/// WASM ブラウザ向けの [`ShareNavigationPort`]。[`share`] の `href` 補助と `history` で Copy link／同期と一致させる。
#[derive(Clone, Copy, Default)]
pub(super) struct WasmShareNavigator;

/// 現在の `location.href` からフラグメント（`#` 以降）を除いた文字列を返す。
fn href_without_fragment() -> Result<String, String> {
    let window = web_sys::window().ok_or("no window")?;
    let loc = window.location();
    // origin+pathname+search は file:// 等で "null/..." を返すことがあるので、href から # だけ切る。
    let href = loc.href().map_err(|_| "href unavailable")?;
    Ok(match href.split_once('#') {
        Some((before, _)) => before.to_string(),
        None => href,
    })
}

/// `location` / `history.replaceState`（失敗時は `location.hash`）で可読フラグメントを読み書きする。
impl ShareNavigationPort for WasmShareNavigator {
    fn full_share_page_url_with_readable_body(
        &self,
        fragment_body: &str,
    ) -> Result<String, String> {
        let base = href_without_fragment()?;
        let base = share::href_with_from_share_query(&base);
        Ok(format!("{base}#{fragment_body}"))
    }

    fn replace_readable_fragment_body(&self, fragment_body: &str) -> Result<(), String> {
        let window = web_sys::window().ok_or("no window")?;
        let loc = window.location();
        let base = href_without_fragment()?;
        let base = share::strip_from_share_query_param(&base);
        let new_url = format!("{base}#{fragment_body}");
        let hist = window.history().map_err(|_| "history unavailable")?;
        let hash_fallback = fragment_body.to_string();
        if hist
            .replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url))
            .is_err()
        {
            loc.set_hash(&hash_fallback)
                .map_err(|_| "set_hash failed")?;
        }
        Ok(())
    }

    fn current_fragment_body(&self) -> Option<String> {
        let window = web_sys::window()?;
        let hash = window.location().hash().ok()?;
        if hash.len() <= 1 {
            return None;
        }
        Some(hash.strip_prefix('#')?.trim().to_string())
    }

    fn readable_share_equals_current_fragment(&self, readable_body: &str) -> bool {
        let Some(head) = self.current_fragment_body() else {
            return false;
        };
        head.starts_with("v=") && head.as_str() == readable_body
    }
}
