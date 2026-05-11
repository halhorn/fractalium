//! `key=value&…` （search・fragment どちらにも載せられる本文）のドメイン意味・検証、および共有 UI 向けの組み立て。
//! 線形表現そのものは [`crate::encoding::flat_query_codec`]（先頭の `?` / `#` は含めない）。

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::sync::Arc;

use bevy::prelude::*;

use crate::app::session::FractalState;
use crate::app::session_rules::clamp_fractal_state_depth;
use crate::core::budget::FRACTAL_DEPTH_HARD_CAP;
use crate::core::shape::{BaseShape, Line, REPLICA_SCALE_MAX, REPLICA_SCALE_MIN, Replica};
use crate::encoding::flat_query_codec::{SubLevel, TopLevel};
use crate::ports::share_link::ShareNavigationPort;

/// 共有クエリのフォーマット版。`v=` と不一致なら復号時に拒否する。
const SHARE_VERSION: u32 = 1;
/// 1 URL に載せる線分の上限（復号時のメモリ・時間を抑える）。
const MAX_LINES: usize = 4096;
/// 1 URL に載せる複製の上限。
const MAX_REPLICAS: usize = 64;

/// 共有ペイロードで許容する `depth` の上限（再帰スタック安全性。操作上限は [`crate::core::budget::max_depth_for_budget`] と組み合わさる）。
pub const MAX_DEPTH: u32 = FRACTAL_DEPTH_HARD_CAP;

/// [`FractalState`] に載せられる幾何・メタが共有フォーマットの制約を満たすか確認する。
/// 版チェック [`SHARE_VERSION`] はクエリ復号側で、`encode` は定数だけ埋める。
///
/// # 引数
/// - `state` — 編集中フラクタルと同一の状態表現。
///
/// # 戻り値
/// 許容なら `Ok(())`。エラー文言は破損クエリ／不正状態のログ向け。
fn validate_fractal_state_readable(state: &FractalState) -> Result<(), String> {
    if state.depth < 1 || state.depth > MAX_DEPTH {
        return Err("depth out of range".into());
    }
    if state.base_shape.lines.len() > MAX_LINES {
        return Err("too many lines".into());
    }
    if state.replicas.len() > MAX_REPLICAS {
        return Err("too many replicas".into());
    }
    for l in &state.base_shape.lines {
        let Line { a, b } = *l;
        if !(a.x.is_finite()
            && a.y.is_finite()
            && b.x.is_finite()
            && b.y.is_finite())
        {
            return Err("non-finite line coordinate".into());
        }
    }
    for r in &state.replicas {
        if !(r.position.x.is_finite()
            && r.position.y.is_finite()
            && r.rotation.is_finite()
            && r.scale.is_finite())
        {
            return Err("non-finite replica field".into());
        }
        if r.scale <= 0.0 {
            return Err("replica scale must be positive".into());
        }
    }
    Ok(())
}

/// [`TopLevel::decode`](crate::encoding::flat_query_codec::TopLevel::decode) でクエリ本文を復号し、`v` / `depth` / … を [`FractalState`] に反映する（深度は [`crate::app::session_rules`] のルールでクランプ）。
///
/// # 引数
/// - `query` — `#` 後の本文全体（`v=` で始まることを期待）。
/// - `state` — 成功時に全体が置き換わる。
///
/// # 戻り値
/// 復号・検証に成功したら `Ok(())`。文言付きの `Err` はユーザー起因の壊れた URL 向け。
pub fn decode_readable_share_query(query: &str, state: &mut FractalState) -> Result<(), String> {
    let mut v = None;
    let mut depth = None;
    let mut show_all = None;
    let mut line_bufs = Vec::<[f32; 4]>::new();
    let mut replicas_buf = Vec::<Replica>::new();

    let top = TopLevel::decode(query)?;
    for (key, val) in top.pairs() {
        match key.as_str() {
            "v" => v = Some(val.decode_to_u32()?),
            "depth" => depth = Some(val.decode_to_u32()?),
            "g" => show_all = Some(val.decode_to_bool_bin()?),
            "line" => line_bufs.push(parse_readable_line(val)?),
            "replica" => replicas_buf.push(parse_readable_replica(val)?),
            _ => { /* 未知キーは無視 */ }
        }
    }

    let v = v.ok_or("missing v")?;
    if v != SHARE_VERSION {
        return Err(format!("unsupported share version {}", v));
    }

    let line_vecs: Vec<Line> = line_bufs
        .into_iter()
        .map(|[ax, ay, bx, by]| Line {
            a: Vec2::new(ax, ay),
            b: Vec2::new(bx, by),
        })
        .collect();

    let new_state = FractalState {
        base_shape: BaseShape { lines: line_vecs },
        replicas: replicas_buf,
        depth: depth.ok_or("missing depth")?,
        show_all_generations: show_all.unwrap_or(false),
    };

    validate_fractal_state_readable(&new_state)?;
    *state = new_state;
    clamp_fractal_state_depth(state);
    Ok(())
}

/// `v=…&depth=…&g=…&line=…&replica=…` 形式のフラグメント本文を生成する。
///
/// # 引数
/// - `state` — 現在のフラクタル状態。
///
/// # 戻り値
/// `#` なしのクエリ文字列。検証に失敗した場合のみ `Err`。
pub fn encode_state(state: &FractalState) -> Result<String, String> {
    validate_fractal_state_readable(state)?;
    let mut pairs: Vec<(String, SubLevel)> = Vec::new();
    pairs.push(("v".into(), SubLevel::new(SHARE_VERSION.to_string())));
    pairs.push(("depth".into(), SubLevel::new(state.depth.to_string())));
    pairs.push((
        "g".into(),
        SubLevel::new(format!("{}", state.show_all_generations as u8)),
    ));
    for l in &state.base_shape.lines {
        pairs.push((
            "line".into(),
            SubLevel::encode_from_f32_vec(&[l.a.x, l.a.y, l.b.x, l.b.y]),
        ));
    }
    for r in &state.replicas {
        pairs.push((
            "replica".into(),
            SubLevel::encode_from_kv_f32(&[
                ("x", r.position.x),
                ("y", r.position.y),
                ("r", r.rotation),
                ("s", r.scale),
            ]),
        ));
    }
    Ok(TopLevel(pairs).encode())
}

/// Web Share に渡す本文。Copy link と同じ状態から [`ShareNavigationPort`] 経由で URL を組み立てる。
///
/// # 引数
/// - `state` — エンコードするフラクタル状態。
/// - `nav` — 完全ページ URL を組み立てるポート実装。
///
/// # 戻り値
/// 常に `Some`。本文は `#fractalium` 行のあとに URL を続けるか、URL 失敗時はタグのみ。
pub fn share_sheet_text_for_export(
    state: &FractalState,
    nav: &Arc<dyn ShareNavigationPort + Send + Sync>,
) -> Option<String> {
    let url = encode_state(state)
        .ok()
        .and_then(|token| nav.full_share_page_url_with_readable_body(&token).ok());
    Some(match url {
        Some(u) => format!("\n#fractalium\n{u}"),
        None => "\n#fractalium".to_string(),
    })
}

/// `line=` のサブレベル値を [`SubLevel::decode_to_f32_vec`] で読み、4 浮動小数であることを検証する。
///
/// # 引数
/// - `val` — `line=` の値側。
///
/// # 戻り値
/// `[ax, ay, bx, by]`。要素数が 4 でない場合は `Err`。
fn parse_readable_line(val: &SubLevel) -> Result<[f32; 4], String> {
    let v = val.decode_to_f32_vec()?;
    if v.len() != 4 {
        return Err(format!("line must have 4 floats, got {}", v.len()));
    }
    Ok([v[0], v[1], v[2], v[3]])
}

/// `replica=` の値を [`SubLevel::decode_to_kv_pairs`] で読み、必須サブキー `x` / `y` / `r` / `s` を [`f32`] で取り出す。
///
/// # 引数
/// - `val` — `replica=` の値側。
///
/// # 戻り値
/// フラクタル定義用の複製。スケールはコアの許容レンジへクランプする。欠損やパース失敗時は `Err`。
fn parse_readable_replica(val: &SubLevel) -> Result<Replica, String> {
    let wire = val.decode_to_kv_pairs()?;
    let x = wire.get_f32("x")?;
    let y = wire.get_f32("y")?;
    let rot = wire.get_f32("r")?;
    let s_wire = wire.get_f32("s")?;
    if s_wire <= 0.0 {
        return Err("replica scale must be positive".into());
    }
    let s = s_wire.clamp(REPLICA_SCALE_MIN, REPLICA_SCALE_MAX);
    Ok(Replica {
        position: Vec2::new(x, y),
        rotation: rot,
        scale: s,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::flat_query_codec::SubLevel;

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
        let r = parse_readable_replica(&SubLevel::new("x:1.0,y:2.0,r:3.0,s:4.0,z:0.0")).unwrap();
        assert_eq!(r.position.x, 1.0);
        assert_eq!(r.position.y, 2.0);
        assert_eq!(r.rotation, 3.0);
        assert_eq!(r.scale, REPLICA_SCALE_MAX);
    }
}
