//! フラクタル本体（Result キャンバス側）の再帰計算と描画を提供するモジュール。
//!
//! 描画手順は次の通り：
//! 1. FractalState が変化したフレームのみ再帰計算を実行
//! 2. 線分集合を LineList Mesh の頂点バッファに書き込む
//! 3. MeshMaterial2d<ColorMaterial> で 1 ドローコールにまとめて描画

use bevy::asset::RenderAssetUsages;
use bevy::color::Hsla;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

use crate::result_layer;
use crate::share::PendingShareUrlSync;
use crate::state::{FractalState, Line, Replica, FRACTAL_DEPTH_HARD_CAP};

/// 1 回の展開でたどる再帰ノード数の上限（`collect_fractal_segments` と同オーダーの訪問回数）。
#[cfg(target_arch = "wasm32")]
const MAX_RECURSION_NODES: u64 = 500_000;
#[cfg(not(target_arch = "wasm32"))]
const MAX_RECURSION_NODES: u64 = 2_000_000;

/// メッシュに載せる線分（基図形の `Line` 1 本 = 1 セグメント）の個数上限。
/// 頂点バッファ・GPU メモリの目安（セグメントあたり位置 2×12B + 色 2×16B 前後）。
#[cfg(target_arch = "wasm32")]
const MAX_LINE_SEGMENTS: u64 = 400_000;
#[cfg(not(target_arch = "wasm32"))]
const MAX_LINE_SEGMENTS: u64 = 1_500_000;

/// 基図形の線数・複製数・「全世代表示」から、予算内で許容する最大 `depth`
/// （`1..=FRACTAL_DEPTH_HARD_CAP` の範囲で、再帰ノード数・線分数が上限以下になる最大値）。
pub fn max_depth_for_budget(
    base_line_count: usize,
    replica_count: usize,
    show_all_generations: bool,
) -> u32 {
    if replica_count == 0 {
        return FRACTAL_DEPTH_HARD_CAP;
    }
    let r = replica_count as u64;
    let l = base_line_count as u64;

    let mut best = 1u32;
    for d in 1..=FRACTAL_DEPTH_HARD_CAP {
        let Some(nodes) = recursion_node_count(r, d) else {
            break;
        };
        if nodes > MAX_RECURSION_NODES {
            break;
        }
        let Some(draws) = line_draw_invocations(r, d, show_all_generations) else {
            break;
        };
        let segments = l.saturating_mul(draws);
        if segments > MAX_LINE_SEGMENTS {
            break;
        }
        best = d;
    }
    best
}

pub fn clamp_fractal_state_depth(state: &mut FractalState) {
    let cap = max_depth_for_budget(
        state.base_shape.lines.len(),
        state.replicas.len(),
        state.show_all_generations,
    );
    state.depth = state.depth.min(cap).max(1);
}

/// `sum_{i=0}^{depth-1} r^i`（完全 r 分木に対する `collect_fractal_segments` の訪問ノード数）。
fn recursion_node_count(r: u64, depth: u32) -> Option<u64> {
    if r == 0 || depth == 0 {
        return None;
    }
    if r == 1 {
        return Some(depth as u64);
    }
    let mut sum = 0u64;
    let mut term = 1u64;
    for _ in 0..depth {
        sum = sum.checked_add(term)?;
        term = term.checked_mul(r)?;
    }
    Some(sum)
}

/// 線分ジオメトリを実際にメッシュへ積む回数（末端のみ / 全世代で各ノード）。
fn line_draw_invocations(r: u64, depth: u32, show_all_generations: bool) -> Option<u64> {
    if depth < 1 {
        return Some(0);
    }
    if show_all_generations {
        recursion_node_count(r, depth)
    } else if depth == 1 {
        Some(1)
    } else {
        let mut t = 1u64;
        for _ in 0..depth - 1 {
            t = t.checked_mul(r)?;
        }
        Some(t)
    }
}

fn clamp_fractal_depth_to_budget(mut state: ResMut<FractalState>) {
    let cap = max_depth_for_budget(
        state.base_shape.lines.len(),
        state.replicas.len(),
        state.show_all_generations,
    );
    if state.depth > cap {
        state.depth = cap;
    }
}

/// `PostUpdate` 内でフラクタルメッシュ更新の前後関係を決める（共有 URL 同期より先に実行する）。
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FractalPostUpdateSet {
    UpdateMesh,
}

/// Result キャンバスへフラクタルを描画する一連のシステムを登録するプラグイン。
pub struct FractalPlugin;

impl Plugin for FractalPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(PostUpdate, FractalPostUpdateSet::UpdateMesh);
        app.add_systems(Startup, setup_fractal_mesh)
            // PostUpdate で実行することで、同フレームの Update（drag 確定）・
            // EguiPrimaryContextPass（DragValue 操作）の変更を即座に反映する。
            // depth の予算 clamp はメッシュ構築の直前に行う（UI パスが Update より後のため）。
            .add_systems(
                PostUpdate,
                (
                    clamp_fractal_depth_to_budget,
                    update_fractal_mesh,
                )
                    .chain()
                    .in_set(FractalPostUpdateSet::UpdateMesh),
            );
    }
}

/// FractalState が変化したフレームのみ Mesh の頂点バッファを更新する。
/// メッシュ書き込み成功後にのみ WASM 向け共有 URL 同期フラグを立てる。
fn update_fractal_mesh(
    state: Res<FractalState>,
    mut pending_share: ResMut<PendingShareUrlSync>,
    mesh_q: Query<&Mesh2d, With<FractalMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !state.is_changed() {
        return;
    }

    let Ok(mesh2d) = mesh_q.single() else {
        return;
    };
    let Some(mesh) = meshes.get_mut(&mesh2d.0) else {
        return;
    };

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();

    collect_fractal_segments(
        state.depth,
        Replica::identity(),
        &state.base_shape.lines,
        &state.replicas,
        &mut positions,
        &mut colors,
        0.0,
        360.0,
        state.show_all_generations,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    pending_share.0 = true;
}

/// フラクタル描画用 Mesh Entity を識別するマーカー。
#[derive(Component)]
struct FractalMesh;

/// HSL 色相からリニア RGBA 配列を生成する。彩度・輝度は固定。
fn hue_to_linear_rgba(hue: f32) -> [f32; 4] {
    LinearRgba::from(Hsla::new(hue % 360.0, 0.88, 0.58, 1.0)).to_f32_array()
}

/// Result パネルの depth=1 親と同じ色（全レプリカを均等に色相分割）。
pub fn result_replica_color(i: usize, total: usize) -> LinearRgba {
    let hue = if total == 0 { 0.0 } else { i as f32 * 360.0 / total as f32 };
    LinearRgba::from(Hsla::new(hue, 0.88, 0.58, 1.0))
}

/// Startup 時に空の LineList Mesh と ColorMaterial を持つ Entity を生成する。
fn setup_fractal_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    commands.spawn((
        FractalMesh,
        Mesh2d(meshes.add(mesh)),
        MeshMaterial2d(materials.add(ColorMaterial::default())),
        result_layer(),
    ));
}

/// フラクタルを再帰的に展開して頂点座標と色を `positions` / `colors` に積む。
///
/// - `depth`: 残りの再帰回数。
/// - `show_all_generations`: true のとき末端だけでなく途中世代も描画する。
fn collect_fractal_segments(
    depth: u32,
    transform: Replica,
    lines: &[Line],
    replicas: &[Replica],
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    hue: f32,
    hue_step: f32,
    show_all_generations: bool,
) {
    let is_leaf = depth <= 1 || replicas.is_empty();

    // 末端、または途中世代も描画する設定のとき、この変換を描画する
    if is_leaf || show_all_generations {
        let color = hue_to_linear_rgba(hue);
        for line in lines {
            let a = transform.apply(line.a);
            let b = transform.apply(line.b);
            positions.push([a.x, a.y, 0.0]);
            positions.push([b.x, b.y, 0.0]);
            colors.push(color);
            colors.push(color);
        }
    }

    if is_leaf {
        return;
    }

    let n = replicas.len() as f32;
    let child_step = hue_step / n;
    for (i, replica) in replicas.iter().enumerate() {
        collect_fractal_segments(
            depth - 1,
            transform.compose(*replica),
            lines,
            replicas,
            positions,
            colors,
            hue + i as f32 * child_step,
            child_step,
            show_all_generations,
        );
    }
}

/// Result キャンバスに描画される線分のワールド座標 AABB。描画ジオメトリが無いときは `None`。
pub fn fractal_world_aabb(state: &FractalState) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    let mut has = false;
    fractal_bounds_recurse(
        state.depth,
        Replica::identity(),
        &state.base_shape.lines,
        &state.replicas,
        &mut min,
        &mut max,
        &mut has,
        state.show_all_generations,
    );
    has.then_some((min, max))
}

fn fractal_bounds_recurse(
    depth: u32,
    transform: Replica,
    lines: &[Line],
    replicas: &[Replica],
    min: &mut Vec2,
    max: &mut Vec2,
    has: &mut bool,
    show_all_generations: bool,
) {
    let is_leaf = depth <= 1 || replicas.is_empty();

    if is_leaf || show_all_generations {
        for line in lines {
            for p in [transform.apply(line.a), transform.apply(line.b)] {
                *min = min.min(p);
                *max = max.max(p);
                *has = true;
            }
        }
    }

    if is_leaf {
        return;
    }

    for replica in replicas {
        fractal_bounds_recurse(
            depth - 1,
            transform.compose(*replica),
            lines,
            replicas,
            min,
            max,
            has,
            show_all_generations,
        );
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    use crate::state::FRACTAL_DEPTH_HARD_CAP;

    #[test]
    fn empty_replicas_allows_abs_max_depth() {
        assert_eq!(
            max_depth_for_budget(100, 0, true),
            FRACTAL_DEPTH_HARD_CAP
        );
    }

    #[test]
    fn high_branching_clamps_below_hard_cap() {
        let d = max_depth_for_budget(1, 8, false);
        assert!(d < FRACTAL_DEPTH_HARD_CAP);
        assert!(d >= 1);
        let nodes = super::recursion_node_count(8, d).expect("nodes");
        assert!(nodes <= MAX_RECURSION_NODES);
    }

    /// 複製が少ない場合は予算内で depth 12 を超えられる（旧 UI 上限の緩和）。
    #[test]
    fn two_replicas_allow_depth_above_twelve_within_budget() {
        let d = max_depth_for_budget(1, 2, false);
        assert!(d > 12);
        assert!(d <= FRACTAL_DEPTH_HARD_CAP);
    }
}
