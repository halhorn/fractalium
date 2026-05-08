//! フラグメント `#<可読クエリ>`（例: `#v=1&depth=…`）でフラクタル状態を共有する。**削除予定の** `#f=<base64url(json)>` の旧形式も読み取れる。
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};


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
    fn validate_constraints(&self) -> Result<(), String> {
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
        for r in &self.replicas {
            if !r.tx.is_finite() || !r.ty.is_finite() || !r.rot.is_finite() || !r.s.is_finite() {
                return Err("non-finite replica field".into());
            }
            if r.s <= 0.0 {
                return Err("replica scale must be positive".into());
            }
        }
        Ok(())
    }

    fn try_into_fractal_state(self) -> Result<FractalState, String> {
        self.validate_constraints()?;
        let mut replicas = Vec::with_capacity(self.replicas.len());
        for r in self.replicas {
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

const SHARE_URL_SIG_FIGS: i32 = 6;
/// これ未満（有効桁丸め後）は座標・回転を URL では `0` にする（計算ギャップ用）。スケール `s` には使わない。
const SHARE_NEAR_ZERO_ABS: f64 = 1e-7;

fn round_to_sig_figs(x: f64, sig: i32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let abs_x = x.abs();
    let log10 = abs_x.log10();
    if !log10.is_finite() {
        return x;
    }
    let m = log10.floor();
    let scale = 10_f64.powf(sig as f64 - 1.0 - m);
    (x * scale).round() / scale
}

fn trim_frac_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// 共有 URL 上の float は常に小数点を含む（`3.0`）。整数だけのトークンにはしない。
fn ensure_share_float_syntax(s: &str) -> String {
    if s == "nan" || s.contains('.') {
        return s.to_string();
    }
    if s.is_empty() || s == "-" {
        "0.0".to_string()
    } else {
        format!("{s}.0")
    }
}

/// 線分座標・replica の x,y,r（および将来の類似項目）。ごく微小値は `0.0`。
fn fmt_share_f32_geom(f: f32) -> String {
    fmt_share_inner(f as f64, true)
}

/// replica の `s`。**正の検証があるため「近いから 0」にはしない。
fn fmt_share_f32_scale(f: f32) -> String {
    fmt_share_inner(f as f64, false)
}

fn fmt_share_inner(v0: f64, clamp_tiny_geom: bool) -> String {
    if !v0.is_finite() {
        return "nan".to_string();
    }
    let v = round_to_sig_figs(v0, SHARE_URL_SIG_FIGS);
    if clamp_tiny_geom && v.abs() < SHARE_NEAR_ZERO_ABS {
        return "0.0".to_string();
    }
    if v == 0.0 {
        return "0.0".to_string();
    }
    let abs_v = v.abs();
    let log10 = abs_v.log10();
    if !log10.is_finite() {
        return "0.0".to_string();
    }
    let m_i = log10.floor() as i32;
    let frac_digits = ((SHARE_URL_SIG_FIGS - 1) - m_i).max(0).min(20) as usize;
    let rendered = format!("{:.*}", frac_digits, v);
    ensure_share_float_syntax(&trim_frac_zeros(&rendered))
}

fn encode_snapshot_readable(snap: &FractalSnapshot) -> Result<String, String> {
    snap.validate_constraints()?;
    let mut pairs = Vec::new();
    pairs.push(format!("v={}", snap.v));
    pairs.push(format!("depth={}", snap.depth));
    pairs.push(format!("g={}", snap.show_all_generations as u8));
    for [ax, ay, bx, by] in &snap.lines {
        pairs.push(format!(
            "line={},{},{},{}",
            fmt_share_f32_geom(*ax),
            fmt_share_f32_geom(*ay),
            fmt_share_f32_geom(*bx),
            fmt_share_f32_geom(*by),
        ));
    }
    for r in &snap.replicas {
        pairs.push(format!(
            "replica=x:{},y:{},r:{},s:{}",
            fmt_share_f32_geom(r.tx),
            fmt_share_f32_geom(r.ty),
            fmt_share_f32_geom(r.rot),
            fmt_share_f32_scale(r.s),
        ));
    }
    Ok(pairs.join("&"))
}

/// 値のみ URL 復号してからパースする（ユーザーがブラウザで自動エンコードした場合）。
fn decoded_pair_value(_key: &str, value: &str) -> Result<String, String> {
    let decoded = urlencoding::decode(value).map_err(|e| e.to_string())?;
    Ok(decoded.into_owned())
}

fn parse_readable_line(seg: &str) -> Result<[f32; 4], String> {
    let parts: Vec<&str> = seg.split(',').collect();
    if parts.len() != 4 {
        return Err(format!("line must have 4 floats, got {}", parts.len()));
    }
    let mut out = [0.0_f32; 4];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("invalid line float: {p}"))?;
    }
    Ok(out)
}

fn parse_readable_replica(seg: &str) -> Result<ReplicaSnapshot, String> {
    let parts: Vec<&str> = seg.split(',').collect();
    if parts.len() < 4 {
        return Err(format!(
            "replica must have x:,y:,r:,s: fields (at least {}, got {})",
            4,
            parts.len()
        ));
    }
    let mut tx = None;
    let mut ty = None;
    let mut rot = None;
    let mut s = None;
    for p in parts {
        let (k, v) = p
            .split_once(':')
            .ok_or_else(|| format!("replica segment missing ':' {p}"))?;
        let v = v.trim();
        match k.trim() {
            "x" => tx = Some(v.parse::<f32>().map_err(|_| format!("bad x:{v}"))?),
            "y" => ty = Some(v.parse::<f32>().map_err(|_| format!("bad y:{v}"))?),
            "r" => rot = Some(v.parse::<f32>().map_err(|_| format!("bad r:{v}"))?),
            "s" => s = Some(v.parse::<f32>().map_err(|_| format!("bad s:{v}"))?),
            _ => {
                /* 将来の拡張フィールドは無視（前方互換） */
            }
        }
    }
    Ok(ReplicaSnapshot {
        tx: tx.ok_or("replica missing x")?,
        ty: ty.ok_or("replica missing y")?,
        rot: rot.ok_or("replica missing r")?,
        s: s.ok_or("replica missing s")?,
    })
}

fn decode_readable_share_query(query: &str, state: &mut FractalState) -> Result<(), String> {
    let mut v = None;
    let mut depth = None;
    let mut show_all = None;
    let mut lines = Vec::<[f32; 4]>::new();
    let mut replicas = Vec::<ReplicaSnapshot>::new();

    for raw_seg in query.split('&') {
        if raw_seg.is_empty() {
            continue;
        }
        let seg = raw_seg.trim();
        let Some((key, raw_val)) = seg.split_once('=') else {
            continue;
        };
        let val = decoded_pair_value(key, raw_val)?;

        match key {
            "v" => {
                v = Some(
                    val.parse::<u32>()
                        .map_err(|_| format!("bad v:{val}"))?,
                )
            }
            "depth" => depth = Some(val.parse::<u32>().map_err(|_| format!("bad depth:{val}"))?),
            "g" => {
                show_all = Some(match val.as_str() {
                    "1" | "true" => true,
                    "0" | "false" => false,
                    _ => return Err(format!("bad g:{val}")),
                })
            }
            "line" => lines.push(parse_readable_line(&val)?),
            "replica" => replicas.push(parse_readable_replica(&val)?),
            _ => {
                /* 未知キーは無視 */
            }
        }
    }

    let snap = FractalSnapshot {
        v: v.ok_or("missing v")?,
        depth: depth.ok_or("missing depth")?,
        show_all_generations: show_all.unwrap_or(false),
        snap_grid: false,
        lines,
        replicas,
    };

    let new_state = snap.try_into_fractal_state()?;
    *state = new_state;
    crate::fractal::clamp_fractal_state_depth(state);
    Ok(())
}

/// `v=…&depth=…&g=…&line=…&replica=…`。フラグメントは `#` の直後にこの文字列を付ける。
pub fn encode_state(state: &FractalState) -> Result<String, String> {
    let snap = FractalSnapshot::from(state);
    encode_snapshot_readable(&snap)
}

/// **Legacy** `#f=<base64url(json)>`. 入力はトークンのみ。**削除予定**。
pub fn decode_and_apply(token: &str, state: &mut FractalState) -> Result<(), String> {
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        token.trim(),
    )
    .map_err(|_| "invalid base64 (legacy)".to_string())?;
    let snap: FractalSnapshot = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let new_state = snap.try_into_fractal_state()?;
    *state = new_state;
    crate::fractal::clamp_fractal_state_depth(state);
    Ok(())
}

/// `Share` からコピーする完全 URL 用: `location` の search に `from=share` を付ける（無い場合のみ）。
fn href_with_from_share_query(base_href_without_fragment: &str) -> String {
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

/// メッシュ同期でアドレスバーを書き換えるとき、`search` の `from=share` を落とす（共有経由ページで編集したあと）。
fn strip_from_share_query_param(base_href_without_fragment: &str) -> String {
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

pub fn share_url_from_token(query: &str) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let base = location_href_without_hash()?;
        let base = href_with_from_share_query(&base);
        Ok(format!("{base}#{query}"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(format!("?from=share#{query}"))
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

/// `#` の直後の可読クエリ（`v=…&…`）。`#f=…` やそれ以外は None。
#[cfg(target_arch = "wasm32")]
fn parse_location_share_query() -> Option<String> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    if hash.len() <= 1 {
        return None;
    }
    let h = hash.strip_prefix('#')?.trim();
    if h.starts_with("f=") {
        return None;
    }
    if h.starts_with("v=") {
        Some(h.to_string())
    } else {
        None
    }
}

/// アドレスバーのフラグメントを現在の共有形式に合わせる（履歴は `replaceState` で置き換え）。
#[cfg(target_arch = "wasm32")]
pub fn set_location_share_query(query: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    let loc = window.location();
    let base = location_href_without_hash()?;
    let base = strip_from_share_query_param(&base);
    let new_url = format!("{base}#{query}");
    let hist = window.history().map_err(|_| "history unavailable")?;
    let hash_fallback = query.to_string();
    if hist
        .replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url))
        .is_err()
    {
        loc.set_hash(&hash_fallback)
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

    let Ok(query) = encode_state(&state) else {
        return;
    };
    if last_token.as_deref() == Some(query.as_str()) {
        return;
    }
    if parse_location_share_query().as_deref() == Some(query.as_str()) {
        *last_token = Some(query);
        return;
    }
    if set_location_share_query(&query).is_ok() {
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
    mut state: ResMut<FractalState>,
    mut undo: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut draw_state: ResMut<DrawState>,
    mut pending_fit: ResMut<PendingResultCameraFit>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(hash) = window.location().hash() else {
        return;
    };
    if hash.len() <= 1 {
        return;
    }
    let Some(h) = hash.strip_prefix('#') else {
        return;
    };
    let h_trim = h.trim();

    let applied = if let Some(rest) = h_trim.strip_prefix("f=") {
        let tok = rest.split('&').next().unwrap_or(rest).trim();
        if tok.is_empty() {
            None
        } else {
            Some(decode_and_apply(tok, &mut *state))
        }
    } else if h_trim.starts_with("v=") {
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
    use base64::Engine;
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
    fn from_share_query_appends_and_dedupes() {
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

    #[test]
    fn round_trip_defaultish() {
        let mut s = FractalState::default();
        s.base_shape.lines.push(Line {
            a: Vec2::new(-0.5, 0.0),
            b: Vec2::new(0.5, 0.0),
        });
        s.replicas.push(Replica::default_new());
        s.depth = 3;
        let q = encode_state(&s).unwrap();
        assert!(q.starts_with("v="), "readable body: {q}");
        let mut out = FractalState::default();
        decode_readable_share_query(&q, &mut out).unwrap();
        assert_eq!(out.depth, 3);
        assert_eq!(out.base_shape.lines.len(), 1);
        assert_eq!(out.replicas.len(), 1);
    }

    #[test]
    fn readable_decode_ignores_unknown_keys() {
        let mut s = FractalState::default();
        s.base_shape.lines.push(Line {
            a: Vec2::new(-0.5, 0.0),
            b: Vec2::new(0.5, 0.0),
        });
        s.replicas.push(Replica::default_new());
        s.depth = 3;
        let mut q = encode_state(&s).unwrap();
        q.push_str("&future_key=abc&version_extra=2");
        let mut out = FractalState::default();
        decode_readable_share_query(&q, &mut out).unwrap();
        assert_eq!(out.depth, 3);
        assert_eq!(out.base_shape.lines.len(), 1);
        assert_eq!(out.replicas.len(), 1);
    }

    #[test]
    fn replica_ignores_unknown_fields() {
        let r = parse_readable_replica("x:1.0,y:2.0,r:3.0,s:4.0,z:0.0").unwrap();
        assert_eq!(r.tx, 1.0);
        assert_eq!(r.ty, 2.0);
        assert_eq!(r.rot, 3.0);
        assert_eq!(r.s, 4.0);
    }

    #[test]
    fn fmt_six_sig_figs_trims_trailing_zeros() {
        assert_eq!(fmt_share_f32_geom(3.10000002f32), "3.1");
    }

    #[test]
    fn fmt_float_always_includes_decimal_point() {
        assert_eq!(fmt_share_f32_geom(3.0f32), "3.0");
        assert_eq!(fmt_share_f32_scale(1.0f32), "1.0");
    }

    #[test]
    fn fmt_geom_tiny_noise_clamps_to_zero() {
        assert_eq!(fmt_share_f32_geom(-0.000000044f32), "0.0");
    }

    #[test]
    fn fmt_scale_keeps_small_positive() {
        let s = fmt_share_f32_scale(0.05f32);
        assert_eq!(s, "0.05");
    }

    #[test]
    fn legacy_base64_decode_still_works() {
        let snap = FractalSnapshot {
            v: SHARE_VERSION,
            depth: 2,
            show_all_generations: true,
            snap_grid: false,
            lines: vec![[-1.0, 0.0, 1.0, 0.0]],
            replicas: vec![ReplicaSnapshot {
                tx: 0.25,
                ty: -0.1,
                rot: 0.5,
                s: 0.75,
            }],
        };
        let json = serde_json::to_vec(&snap).unwrap();
        let tok = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&json);
        let mut out = FractalState::default();
        decode_and_apply(&tok, &mut out).unwrap();
        assert_eq!(out.depth, 2);
        assert!(out.show_all_generations);
        assert_eq!(out.base_shape.lines.len(), 1);
        assert_eq!(out.replicas.len(), 1);
    }
}
