//! `key=value&…` （search・fragment どちらにも載せられる本文）のドメイン意味・検証、および共有 UI 向けの組み立て。
//! 線形表現そのものは [`crate::encoding::flat_query_codec`]（先頭の `?` / `#` は含めない）。

use std::sync::Arc;

use bevy::prelude::*;

use super::workspace::clamp_fractal_state_depth;
use crate::encoding::flat_query_codec::{SubLevel, TopLevel};
use crate::ports::share_link::ShareNavigationPort;
use crate::state::{
    BaseShape, FRACTAL_DEPTH_HARD_CAP, FractalState, Line, REPLICA_SCALE_MAX, REPLICA_SCALE_MIN,
    Replica,
};

/// 共有クエリのフォーマット版。`v=` と不一致なら復号時に拒否する。
const SHARE_VERSION: u32 = 1;
/// 1 URL に載せる線分の上限（復号時のメモリ・時間を抑える）。
const MAX_LINES: usize = 4096;
/// 1 URL に載せる複製の上限。
const MAX_REPLICAS: usize = 64;

/// 共有ペイロードで許容する `depth` の上限（再帰スタック安全性。操作上限は [`crate::core::budget::max_depth_for_budget`] と組み合わさる）。
pub const MAX_DEPTH: u32 = FRACTAL_DEPTH_HARD_CAP;

/// `encode` / `decode` / `validate` で共通する、フラグメント向けのスナップショット。
///
/// `FractalState` とほぼ同じ情報だが、線分は `[ax,ay,bx,by]` のフラット配列で持つ。
struct FractalSnapshot {
    /// [`SHARE_VERSION`] と一致しなければ復号しない。
    v: u32,
    /// 再帰の深さ。
    depth: u32,
    /// 途中世代も描くか。
    show_all_generations: bool,
    /// グリッドスナップのオン／オフ。
    snap_grid: bool,
    /// 基図形の各線分を `[ax, ay, bx, by]` で並べたもの。
    lines: Vec<[f32; 4]>,
    /// 各複製変換。
    replicas: Vec<ReplicaSnapshot>,
}

/// [`Replica`] をクエリの `replica=x:,y:,…` に載せるための中間表現。
struct ReplicaSnapshot {
    /// 平行移動 X。
    x: f32,
    /// 平行移動 Y。
    y: f32,
    /// 回転（ラジアン）。
    rot: f32,
    /// スケール（正の値として検証される）。
    s: f32,
}

impl From<&FractalState> for FractalSnapshot {
    /// 現在のワークスペース状態から共有スナップショットを組み立てる。
    ///
    /// # 引数
    /// - `state` — ソース状態。
    ///
    /// # 戻り値
    /// [`SHARE_VERSION`] を埋めたスナップショット。
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
                    x: r.position.x,
                    y: r.position.y,
                    rot: r.rotation,
                    s: r.scale,
                })
                .collect(),
        }
    }
}

impl FractalSnapshot {
    /// 版・件数・有限性・スケール正などを検証する。エラー文言は復号失敗のログ向け。
    ///
    /// # 戻り値
    /// すべて通れば `Ok(())`。
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
            if !r.x.is_finite() || !r.y.is_finite() || !r.rot.is_finite() || !r.s.is_finite() {
                return Err("non-finite replica field".into());
            }
            if r.s <= 0.0 {
                return Err("replica scale must be positive".into());
            }
        }
        Ok(())
    }

    /// 検証済みスナップショットを [`FractalState`] に変換する。複製スケールはコアの許容範囲へクランプする。
    ///
    /// # 戻り値
    /// ワークスペースへ載せられる状態。検証に失敗した場合は `Err`。
    fn try_into_fractal_state(self) -> Result<FractalState, String> {
        self.validate_constraints()?;
        let mut replicas = Vec::with_capacity(self.replicas.len());
        for r in self.replicas {
            let s = r.s.clamp(REPLICA_SCALE_MIN, REPLICA_SCALE_MAX);
            replicas.push(Replica {
                position: Vec2::new(r.x, r.y),
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

/// [`TopLevel::decode`](crate::encoding::flat_query_codec::TopLevel::decode) でクエリ本文を復号し、`v` / `depth` / … を [`FractalState`] に反映する（深度はワークスペースルールでクランプ）。
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
    let mut lines = Vec::<[f32; 4]>::new();
    let mut replicas = Vec::<ReplicaSnapshot>::new();

    let top = TopLevel::decode(query)?;
    for (key, val) in top.pairs() {
        match key.as_str() {
            "v" => v = Some(val.decode_to_u32()?),
            "depth" => depth = Some(val.decode_to_u32()?),
            "g" => show_all = Some(val.decode_to_bool_bin()?),
            "line" => lines.push(parse_readable_line(val)?),
            "replica" => replicas.push(parse_readable_replica(val)?),
            _ => { /* 未知キーは無視 */ }
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
    clamp_fractal_state_depth(state);
    Ok(())
}

/// `v=…&depth=…&g=…&line=…&replica=…` 形式のフラグメント本文を生成する。
///
/// # 引数
/// - `state` — 現在のワークスペース状態。
///
/// # 戻り値
/// `#` なしのクエリ文字列。検証に失敗した場合のみ `Err`。
pub fn encode_state(state: &FractalState) -> Result<String, String> {
    let snap = FractalSnapshot::from(state);
    encode_snapshot_readable(&snap)
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

/// [`FractalSnapshot`] をフラットクエリへ載せる。[`TopLevel::encode`](crate::encoding::flat_query_codec::TopLevel::encode) で本文を連結する。
///
/// # 引数
/// - `snap` — 検証・シリアライズするスナップショット。
///
/// # 戻り値
/// 成功時はフラグメント本文（先頭の `#` は付けない）。検証失敗時は `Err`。
fn encode_snapshot_readable(snap: &FractalSnapshot) -> Result<String, String> {
    snap.validate_constraints()?;
    let mut pairs: Vec<(String, SubLevel)> = Vec::new();
    pairs.push(("v".into(), SubLevel::new(snap.v.to_string())));
    pairs.push(("depth".into(), SubLevel::new(snap.depth.to_string())));
    pairs.push((
        "g".into(),
        SubLevel::new(format!("{}", snap.show_all_generations as u8)),
    ));
    for [ax, ay, bx, by] in &snap.lines {
        pairs.push((
            "line".into(),
            SubLevel::encode_from_f32_vec(&[*ax, *ay, *bx, *by]),
        ));
    }
    for r in &snap.replicas {
        pairs.push((
            "replica".into(),
            SubLevel::encode_from_kv_f32(&[("x", r.x), ("y", r.y), ("r", r.rot), ("s", r.s)]),
        ));
    }
    Ok(TopLevel(pairs).encode())
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
/// [`ReplicaSnapshot`]。欠損やパース失敗時は `Err`。
fn parse_readable_replica(val: &SubLevel) -> Result<ReplicaSnapshot, String> {
    let wire = val.decode_to_kv_pairs()?;
    Ok(ReplicaSnapshot {
        x: wire.get_f32("x")?,
        y: wire.get_f32("y")?,
        rot: wire.get_f32("r")?,
        s: wire.get_f32("s")?,
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
        assert_eq!(r.x, 1.0);
        assert_eq!(r.y, 2.0);
        assert_eq!(r.rot, 3.0);
        assert_eq!(r.s, 4.0);
    }
}
