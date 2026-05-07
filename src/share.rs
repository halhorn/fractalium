//! `#f=<base64url(json)>` 形式の URL フラグメントでフラクタル状態を共有する（サーバー不要）。
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

#[cfg(target_arch = "wasm32")]
use crate::edit::DrawState;
use crate::state::{
    BaseShape, FRACTAL_DEPTH_HARD_CAP, FractalState, Line, REPLICA_SCALE_MAX, REPLICA_SCALE_MIN,
    Replica,
};

#[cfg(target_arch = "wasm32")]
use crate::state::{PendingResultCameraFit, PlacementDrag, PlacementState, UndoStack};

const SHARE_VERSION: u32 = 1;
const MAX_LINES: usize = 4096;
const MAX_REPLICAS: usize = 64;
/// 共有ペイロードで許容する `depth` の上限（再帰スタック安全性。実際の操作上限は `fractal::max_depth_for_budget` 側の予算）。
pub const MAX_DEPTH: u32 = FRACTAL_DEPTH_HARD_CAP;

#[derive(Serialize, Deserialize)]
struct FractalSnapshot {
    v: u32,
    depth: u32,
    show_all_generations: bool,
    snap_grid: bool,
    lines: Vec<[f32; 4]>,
    replicas: Vec<ReplicaSnapshot>,
}

#[derive(Serialize, Deserialize)]
struct ReplicaSnapshot {
    tx: f32,
    ty: f32,
    rot: f32,
    s: f32,
}

impl From<&FractalState> for FractalSnapshot {
    fn from(state: &FractalState) -> Self {
        Self {
            v: SHARE_VERSION,
            depth: state.depth,
            show_all_generations: state.show_all_generations,
            snap_grid: state.snap_grid,
            lines: state
                .base_shape
                .lines
                .iter()
                .map(|l| [l.a.x, l.a.y, l.b.x, l.b.y])
                .collect(),
            replicas: state
                .replicas
                .iter()
                .map(|r| ReplicaSnapshot {
                    tx: r.translation.x,
                    ty: r.translation.y,
                    rot: r.rotation,
                    s: r.scale,
                })
                .collect(),
        }
    }
}

impl FractalSnapshot {
    fn try_into_fractal_state(self) -> Result<FractalState, String> {
        if self.v != SHARE_VERSION {
            return Err(format!("unsupported share version {}", self.v));
        }
        if self.depth < 1 || self.depth > MAX_DEPTH {
            return Err("depth out of range".into());
        }
        if self.lines.len() > MAX_LINES {
            return Err("too many lines".into());
        }
        if self.replicas.len() > MAX_REPLICAS {
            return Err("too many replicas".into());
        }
        for [ax, ay, bx, by] in &self.lines {
            if !ax.is_finite() || !ay.is_finite() || !bx.is_finite() || !by.is_finite() {
                return Err("non-finite line coordinate".into());
            }
        }
        let mut replicas = Vec::with_capacity(self.replicas.len());
        for r in self.replicas {
            if !r.tx.is_finite() || !r.ty.is_finite() || !r.rot.is_finite() || !r.s.is_finite() {
                return Err("non-finite replica field".into());
            }
            if r.s <= 0.0 {
                return Err("replica scale must be positive".into());
            }
            let s = r.s.clamp(REPLICA_SCALE_MIN, REPLICA_SCALE_MAX);
            replicas.push(Replica {
                translation: Vec2::new(r.tx, r.ty),
                rotation: r.rot,
                scale: s,
            });
        }
        let lines: Vec<Line> = self
            .lines
            .into_iter()
            .map(|[ax, ay, bx, by]| Line {
                a: Vec2::new(ax, ay),
                b: Vec2::new(bx, by),
            })
            .collect();

        Ok(FractalState {
            base_shape: BaseShape { lines },
            replicas,
            depth: self.depth,
            show_all_generations: self.show_all_generations,
            snap_grid: self.snap_grid,
        })
    }
}

/// 共有用ペイロード（URL-safe Base64、パディングなし）。
pub fn encode_state(state: &FractalState) -> Result<String, String> {
    let snap = FractalSnapshot::from(state);
    let json = serde_json::to_vec(&snap).map_err(|e| e.to_string())?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        json,
    ))
}

pub fn decode_and_apply(token: &str, state: &mut FractalState) -> Result<(), String> {
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        token.trim(),
    )
    .map_err(|_| "invalid base64".to_string())?;
    let snap: FractalSnapshot = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let new_state = snap.try_into_fractal_state()?;
    *state = new_state;
    crate::fractal::clamp_fractal_state_depth(state);
    Ok(())
}

pub fn share_url_from_token(token: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let base = location_href_without_hash()?;
        Ok(format!("{base}#f={token}"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(format!("#f={token}"))
    }
}

#[cfg(target_arch = "wasm32")]
fn location_href_without_hash() -> Result<String, String> {
    let window = web_sys::window().ok_or("no window")?;
    let loc = window.location();
    // `origin + pathname + search` は file:// 等で origin が JS 上 "null" になり
    // `"null/..."` という無効 URL になることがある。href からフラグメントだけ落とす。
    let href = loc.href().map_err(|_| "href unavailable")?;
    let base = match href.split_once('#') {
        Some((before, _)) => before.to_string(),
        None => href,
    };
    Ok(base)
}

#[cfg(target_arch = "wasm32")]
fn parse_location_hash_token() -> Option<String> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    let h = hash.strip_prefix('#')?;
    let rest = h.strip_prefix("f=")?;
    let token = rest.split('&').next()?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// アドレスバーのフラグメントを現在の共有形式に合わせる（履歴は `replaceState` で置き換え）。
#[cfg(target_arch = "wasm32")]
pub fn set_location_share_token(token: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    let loc = window.location();
    let base = location_href_without_hash()?;
    let new_url = format!("{base}#f={token}");
    let hist = window.history().map_err(|_| "history unavailable")?;
    if hist
        .replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url))
        .is_err()
    {
        loc.set_hash(&format!("f={token}"))
            .map_err(|_| "set_hash failed")?;
    }
    Ok(())
}

/// Result メッシュを実際に更新したフレームだけ立て、`flush_share_url_after_fractal_mesh` が URL を同期する。
#[derive(Resource, Default)]
pub(crate) struct PendingShareUrlSync(pub bool);

#[cfg(target_arch = "wasm32")]
fn flush_share_url_after_fractal_mesh(
    state: Res<FractalState>,
    mut pending: ResMut<PendingShareUrlSync>,
    mut last_token: Local<Option<String>>,
) {
    if !pending.0 {
        return;
    }
    pending.0 = false;

    let Ok(token) = encode_state(&state) else {
        return;
    };
    if last_token.as_deref() == Some(token.as_str()) {
        return;
    }
    if parse_location_hash_token().as_deref() == Some(token.as_str()) {
        *last_token = Some(token);
        return;
    }
    if set_location_share_token(&token).is_ok() {
        *last_token = Some(token);
    }
}

// --- WASM: v.gd で共有 URL を短くしてからクリップボードへ（直前フレームの fetch 結果を 1 システムで処理）

const VGD_CREATE: &str = "https://v.gd/create.php";

/// v.gd はループバック・プライベート IP・`localhost` ホストを短縮対象にしない。
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn vgd_accepts_share_page_url(full_url: &str) -> bool {
    use std::net::{Ipv4Addr, Ipv6Addr};

    let base = full_url.split('#').next().unwrap_or(full_url);
    let rest = match base
        .strip_prefix("http://")
        .or_else(|| base.strip_prefix("https://"))
    {
        Some(r) => r,
        None => return false,
    };
    let before_path = rest.split(&['/', '?'][..]).next().unwrap_or(rest);
    let before_path = before_path
        .rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(before_path);
    let host = if let (Some(l), Some(r)) = (before_path.find('['), before_path.find(']')) {
        before_path.get(l + 1..r).unwrap_or("").to_string()
    } else if let Some((h, tail)) = before_path.rsplit_once(':') {
        if tail.chars().all(|c| c.is_ascii_digit()) && !h.contains(']') {
            h.to_string()
        } else {
            before_path.to_string()
        }
    } else {
        before_path.to_string()
    };
    let host = host.to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") {
        return false;
    }
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return !ip.is_loopback() && !ip.is_private() && !ip.is_link_local();
    }
    if let Ok(ip) = host.parse::<Ipv6Addr>() {
        return !ip.is_loopback() && !ip.is_unicast_link_local();
    }
    true
}

#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
pub struct ShareShortenWasm {
    pub(crate) pending: Arc<Mutex<Option<Result<String, String>>>>,
    pub busy: bool,
}

#[cfg(target_arch = "wasm32")]
impl Default for ShareShortenWasm {
    fn default() -> Self {
        Self {
            pending: Arc::new(Mutex::new(None)),
            busy: false,
        }
    }
}

/// Share のクリックに渡す参照束ね（ブラウザ版のみ）。
#[cfg(target_arch = "wasm32")]
pub struct ShareWasmBundle<'a> {
    pub wasm: &'a mut ShareShortenWasm,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
pub struct ShareWasmBundle<'a> {
    _p: core::marker::PhantomData<*const &'a ()>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
struct VgdJson {
    shorturl: Option<String>,
    errormessage: Option<String>,
}

#[cfg(target_arch = "wasm32")]
async fn wasm_vgd_post_form(body: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(body));
    let headers = Headers::new().map_err(|_| "headers init failed".to_string())?;
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|_| "Content-Type header".to_string())?;
    opts.set_headers(&headers);
    let req = Request::new_with_str_and_init(VGD_CREATE, &opts)
        .map_err(|_| "request build".to_string())?;
    let js_resp = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|_| "network error".to_string())?;
    let resp: Response = js_resp
        .dyn_into()
        .map_err(|_| "invalid fetch response".to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = JsFuture::from(
        resp.text()
            .map_err(|_| "response has no body".to_string())?,
    )
    .await
    .map_err(|_| "could not read body".to_string())?;
    text.as_string().ok_or_else(|| "empty body".to_string())
}

#[cfg(target_arch = "wasm32")]
async fn shorten_via_vgd(long_url: &str) -> Result<String, String> {
    if !vgd_accepts_share_page_url(long_url) {
        return Ok(long_url.to_string());
    }
    let encoded = urlencoding::encode(long_url);
    let body = format!("format=json&url={encoded}");
    let text = wasm_vgd_post_form(&body).await?;
    let parsed: VgdJson = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if let Some(s) = parsed.shorturl.filter(|u| !u.is_empty()) {
        return Ok(s);
    }
    if let Some(m) = parsed.errormessage {
        return Err(m);
    }
    Err("unexpected v.gd response".to_string())
}

/// `true` なら短縮処理をキューした（`false` は既に処理中）。
#[cfg(target_arch = "wasm32")]
pub fn request_share_short_link_start(wasm: &mut ShareShortenWasm, long_url: String) -> bool {
    if wasm.busy {
        return false;
    }
    wasm.busy = true;
    let sink = wasm.pending.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let out = shorten_via_vgd(&long_url).await;
        if let Ok(mut slot) = sink.lock() {
            *slot = Some(out);
        }
    });
    true
}

pub struct SharePlugin;

impl Plugin for SharePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingShareUrlSync>();
        #[cfg(target_arch = "wasm32")]
        {
            app.init_resource::<ShareShortenWasm>();
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
    mut state: ResMut<FractalState>,
    mut undo: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut draw_state: ResMut<DrawState>,
    mut pending_fit: ResMut<PendingResultCameraFit>,
) {
    let Some(token) = parse_location_hash_token() else {
        return;
    };
    if decode_and_apply(&token, &mut state).is_ok() {
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
    fn vgd_accepts_public_site() {
        assert!(vgd_accepts_share_page_url(
            "https://example.com/app#f=token"
        ));
    }

    #[test]
    fn vgd_rejects_loopback() {
        assert!(!vgd_accepts_share_page_url("http://127.0.0.1:8080/#f=ab"));
    }

    #[test]
    fn round_trip_defaultish() {
        let mut s = FractalState::default();
        s.base_shape.lines.push(Line {
            a: Vec2::new(-0.5, 0.0),
            b: Vec2::new(0.5, 0.0),
        });
        s.replicas.push(Replica::default_new());
        s.depth = 3;
        let t = encode_state(&s).unwrap();
        let mut out = FractalState::default();
        decode_and_apply(&t, &mut out).unwrap();
        assert_eq!(out.depth, 3);
        assert_eq!(out.base_shape.lines.len(), 1);
        assert_eq!(out.replicas.len(), 1);
    }
}
