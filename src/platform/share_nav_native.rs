//! （ネイティブ専用）[`crate::ports::share_link::ShareNavigationPort`] の実装。ウィンドウアプリには URL バーがないため、フラグメントの読み書きはスタブとなる。
//!
//! 「Copy link」用に `?from=share#…` 形式だけ返し、現在のフラグメント照会は常に無い。
use crate::ports::share_link::ShareNavigationPort;

/// ネイティブ向けの [`ShareNavigationPort`]。ブラウザの URL はないためフラグメント操作はすべて空振りになる。
#[derive(Clone, Copy, Default)]
pub(super) struct DesktopShareNavigator;

/// 共有クエリだけをクエリパラメータと `#` で整形し、アドレスバー相当の状態は読めない／書けないスタブ実装。
impl ShareNavigationPort for DesktopShareNavigator {
    fn full_share_page_url_with_readable_body(
        &self,
        fragment_body: &str,
    ) -> Result<String, String> {
        Ok(format!("?from=share#{fragment_body}"))
    }

    fn replace_readable_fragment_body(&self, _fragment_body: &str) -> Result<(), String> {
        Ok(())
    }

    fn current_fragment_body(&self) -> Option<String> {
        None
    }

    fn readable_share_equals_current_fragment(&self, _readable_body: &str) -> bool {
        false
    }
}
