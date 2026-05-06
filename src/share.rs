//! `#f=<base64url(json)>` 形式の URL フラグメントでフラクタル状態を共有する（サーバー不要）。
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use crate::edit::DrawState;
use crate::state::{
    BaseShape, FractalState, Line, Replica, REPLICA_SCALE_MAX, REPLICA_SCALE_MIN,
};

#[cfg(target_arch = "wasm32")]
use crate::state::{PendingResultCameraFit, PlacementDrag, PlacementState, UndoStack};

const SHARE_VERSION: u32 = 1;
const MAX_LINES: usize = 4096;
const MAX_REPLICAS: usize = 64;
const MAX_DEPTH: u32 = 12;

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
    Ok(
        base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            json,
        ),
    )
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
    let origin = loc.origin().map_err(|_| "origin unavailable")?;
    let path = loc.pathname().map_err(|_| "pathname unavailable")?;
    let search = loc.search().map_err(|_| "search unavailable")?;
    Ok(format!("{origin}{path}{search}"))
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

pub struct SharePlugin;

impl Plugin for SharePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Startup, hydrate_from_url);
        #[cfg(not(target_arch = "wasm32"))]
        let _ = app;
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
