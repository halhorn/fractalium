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
use crate::state::{FractalState, Line, Replica};

/// Result キャンバスへフラクタルを描画する一連のシステムを登録するプラグイン。
pub struct FractalPlugin;

impl Plugin for FractalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_fractal_mesh)
            // PostUpdate で実行することで、同フレームの Update（drag 確定）・
            // EguiPrimaryContextPass（DragValue 操作）の変更を即座に反映する。
            .add_systems(PostUpdate, update_fractal_mesh);
    }
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

/// FractalState が変化したフレームのみ Mesh の頂点バッファを更新する。
fn update_fractal_mesh(
    state: Res<FractalState>,
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
