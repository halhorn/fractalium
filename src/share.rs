//! フラグメント `#<可読クエリ>` による URL 同期と Bevy プラグイン。
//!
//! クエリ本文の符号化は [`crate::encoding::flat_query_codec`]、ドメイン検証は [`crate::app`]。
//! `from=share` のようなナビゲーション用 search の操作もこのモジュールに置く（フラグメント本体とは別）。

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::sync::Arc;

use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::edit::DrawState;
#[cfg(target_arch = "wasm32")]
use crate::platform_handles::PlatformHandles;
use crate::ports::share_link::ShareNavigationPort;

#[cfg(target_arch = "wasm32")]
use crate::state::{
    FractalState, PendingResultCameraFit, PlacementDrag, PlacementState, UndoStack,
};

#[allow(unused_imports)]
pub use crate::app::MAX_DEPTH;
/// 旧パス互換。本体実装は [`crate::app::fractal_share`]。
#[cfg_attr(not(target_arch = "wasm32"), allow(unused_imports))]
pub use crate::app::{decode_readable_share_query, encode_state};

/// Copy link 用に、`location` の search に `from=share` を 1 回だけ足す（既にあるときは何もしない）。
///
/// `base` は `#` を含まない `href` のプレフィックス（origin+pathname+search）を想定。
pub(crate) fn href_with_from_share_query(base_href_without_fragment: &str) -> String {
    if query_has_from_share(base_href_without_fragment) {
        return base_href_without_fragment.to_string();
    }
    if let Some((path, q)) = base_href_without_fragment.split_once('?') {
        format!("{path}?{q}&from=share")
    } else {
        format!("{base_href_without_fragment}?from=share")
    }
}

fn query_has_from_share(href: &str) -> bool {
    let Some((_path, query)) = href.split_once('?') else {
        return false;
    };
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let Some(name) = parts.next() else {
            continue;
        };
        if name != "from" {
            continue;
        }
        let value = parts.next().unwrap_or("");
        if value == "share" {
            return true;
        }
    }
    false
}

/// メッシュ同期でアドレスバーを書き換えるとき、`search` の `from=share` を落とす。
pub(crate) fn strip_from_share_query_param(base_href_without_fragment: &str) -> String {
    let Some((path, query)) = base_href_without_fragment.split_once('?') else {
        return base_href_without_fragment.to_string();
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| {
            let mut parts = pair.splitn(2, '=');
            let Some(name) = parts.next() else {
                return true;
            };
            let value = parts.next().unwrap_or("");
            !(name == "from" && value == "share")
        })
        .collect();
    if kept.is_empty() {
        path.to_string()
    } else {
        format!("{}?{}", path, kept.join("&"))
    }
}

/// メッシュ確定後の URL 同期などに使う環境側ナビゲーション。具象は `platform`、`main` で注入する。
#[derive(Resource, Clone)]
pub struct ShareNavigation(pub Arc<dyn ShareNavigationPort + Send + Sync>);

pub fn share_url_from_token(nav: &ShareNavigation, query: &str) -> Result<String, String> {
    nav.0.full_share_page_url_with_readable_body(query)
}

/// Result メッシュを実際に更新したフレームだけ立て、`flush_share_url_after_fractal_mesh` が URL を同期する。
#[derive(Resource, Default)]
pub(crate) struct PendingShareUrlSync(pub bool);

#[cfg(target_arch = "wasm32")]
fn flush_share_url_after_fractal_mesh(
    state: Res<FractalState>,
    platform: Res<PlatformHandles>,
    mut pending: ResMut<PendingShareUrlSync>,
    mut last_token: Local<Option<String>>,
) {
    if !pending.0 {
        return;
    }
    pending.0 = false;

    let Ok(query) = encode_state(&state) else {
        return;
    };
    if last_token.as_deref() == Some(query.as_str()) {
        return;
    }
    let nav = &platform.share_navigation;
    if nav.0.readable_share_equals_current_fragment(query.as_str()) {
        *last_token = Some(query);
        return;
    }
    if nav.0.replace_readable_fragment_body(&query).is_ok() {
        *last_token = Some(query);
    }
}

pub struct SharePlugin;

impl Plugin for SharePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingShareUrlSync>();
        #[cfg(target_arch = "wasm32")]
        {
            app.add_systems(Startup, hydrate_from_url);
            app.add_systems(
                PostUpdate,
                flush_share_url_after_fractal_mesh
                    .after(crate::fractal::FractalPostUpdateSet::UpdateMesh),
            );
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn hydrate_from_url(
    platform: Res<PlatformHandles>,
    mut state: ResMut<FractalState>,
    mut undo: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut draw_state: ResMut<DrawState>,
    mut pending_fit: ResMut<PendingResultCameraFit>,
) {
    let Some(h_trim_owned) = platform.share_navigation.0.current_fragment_body() else {
        return;
    };
    let h_trim = h_trim_owned.trim();

    let applied = if h_trim.starts_with("v=") {
        Some(decode_readable_share_query(h_trim, &mut *state))
    } else {
        None
    };

    if applied.is_some_and(|r| r.is_ok()) {
        undo.clear();
        placement.selected = None;
        placement.clipboard = None;
        placement.drag = PlacementDrag::Idle;
        *draw_state = DrawState::Idle;
        pending_fit.0 = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_from_share_query_param_removes_only_marker() {
        assert_eq!(
            strip_from_share_query_param("https://x.example/y"),
            "https://x.example/y"
        );
        assert_eq!(
            strip_from_share_query_param("https://x.example/y?from=share"),
            "https://x.example/y"
        );
        assert_eq!(
            strip_from_share_query_param("https://x.example/y?a=1&from=share"),
            "https://x.example/y?a=1"
        );
        assert_eq!(
            strip_from_share_query_param("https://x.example/y?from=share&a=1"),
            "https://x.example/y?a=1"
        );
    }

    #[test]
    fn href_with_from_share_appends_and_dedupes() {
        assert_eq!(
            href_with_from_share_query("https://example.com/app"),
            "https://example.com/app?from=share"
        );
        assert_eq!(
            href_with_from_share_query("https://example.com/app?x=1"),
            "https://example.com/app?x=1&from=share"
        );
        assert_eq!(
            href_with_from_share_query("https://example.com/app?from=share"),
            "https://example.com/app?from=share"
        );
    }
}
