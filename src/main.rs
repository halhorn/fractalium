mod core;
mod edit;
mod fractal;
mod fractal_presets;
mod grid;
mod placement;
mod result_export;
mod seed_shape;
mod share;
mod state;
mod toast;
mod ui;
mod view;

use bevy::camera::{OrthographicProjection, Projection, ScalingMode, visibility::RenderLayers};
use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, EguiPlugin, PrimaryEguiContext};

use edit::EditPlugin;
use fractal::{FractalPlugin, clamp_fractal_state_depth};
use fractal_presets::FractalPreset;
use placement::PlacementPlugin;
use result_export::ResultExportPlugin;
use share::SharePlugin;
use state::{CanvasLayout, FractalState, UiLayout};
use ui::UiPlugin;
use view::ViewPlugin;

/// Edit キャンバスを描画するカメラのマーカーコンポーネント。
#[derive(Component)]
pub struct EditCamera;

/// Placement キャンバスを描画するカメラのマーカーコンポーネント。
#[derive(Component)]
pub struct PlacementCamera;

/// Result キャンバスを描画するカメラのマーカーコンポーネント。
#[derive(Component)]
pub struct ResultCamera;

/// Edit キャンバス上のオブジェクトを描画するレンダーレイヤ。
pub fn edit_layer() -> RenderLayers {
    RenderLayers::layer(1)
}

/// Placement キャンバス上のオブジェクトを描画するレンダーレイヤ。
pub fn placement_layer() -> RenderLayers {
    RenderLayers::layer(3)
}

/// Result キャンバス上のオブジェクトを描画するレンダーレイヤ。
pub fn result_layer() -> RenderLayers {
    RenderLayers::layer(2)
}

/// Result と独立したレイヤで、PNG 用の太線フラクタルだけを書き込むカメラ向け。
pub fn result_export_layer() -> RenderLayers {
    RenderLayers::layer(6)
}

fn initial_fractal_state() -> FractalState {
    let mut s = if let Ok(raw) = std::env::var("FRACTALIUM_BOOT_PRESET") {
        if let Some(preset) = FractalPreset::from_boot_token(&raw) {
            preset.build()
        } else {
            FractalState::default()
        }
    } else {
        FractalState::default()
    };
    clamp_fractal_state_depth(&mut s);
    s
}

#[cfg(target_arch = "wasm32")]
fn hide_web_loading_overlay() {
    let Some(win) = web_sys::window() else {
        return;
    };
    let Some(doc) = win.document() else {
        return;
    };
    let Some(el) = doc.get_element_by_id("fractalium-loading") else {
        return;
    };
    let _ = el.set_attribute("hidden", "");
    let _ = el.set_attribute("aria-busy", "false");
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Fractalium".to_string(),
                resolution: (1280, 720).into(),
                canvas: Some("#fractalium-canvas".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins((
            EditPlugin,
            FractalPlugin,
            PlacementPlugin,
            ViewPlugin,
            SharePlugin,
            ResultExportPlugin,
            UiPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.10)))
        .insert_resource(initial_fractal_state())
        .insert_resource(CanvasLayout::default())
        .insert_resource(UiLayout::default())
        .add_systems(Startup, setup)
        .run();
}

/// 単位正方形 [-1, 1] x [-1, 1] が常に画面内に収まる正規化直交投影を返す。
fn normalized_projection() -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::AutoMin {
            min_width: 2.0,
            min_height: 2.0,
        },
        ..OrthographicProjection::default_2d()
    })
}

/// 起動時にカメラ・装飾スプライトを生成する。
fn setup(mut commands: Commands, mut egui_global: ResMut<EguiGlobalSettings>) {
    egui_global.auto_create_primary_context = false;

    spawn_world_cameras(&mut commands);
    spawn_egui_camera(&mut commands);
    spawn_canvas_decor(&mut commands, edit_layer());
    spawn_canvas_decor(&mut commands, placement_layer());

    #[cfg(target_arch = "wasm32")]
    hide_web_loading_overlay();
}

/// Edit / Placement / Result それぞれのキャンバスを描画する 2D カメラを生成する。
fn spawn_world_cameras(commands: &mut Commands) {
    commands.spawn((
        EditCamera,
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        normalized_projection(),
        edit_layer(),
    ));
    commands.spawn((
        PlacementCamera,
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        normalized_projection(),
        placement_layer(),
    ));
    commands.spawn((
        ResultCamera,
        Camera2d,
        Camera {
            order: 2,
            ..default()
        },
        normalized_projection(),
        result_layer(),
    ));
}

/// ワールドを描画せず UI だけをウィンドウ全体に重ねる専用カメラを生成する。
fn spawn_egui_camera(commands: &mut Commands) {
    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        RenderLayers::none(),
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

/// 指定レイヤに正方形の枠と原点十字マーカーを生成する。
fn spawn_canvas_decor(commands: &mut Commands, layer: RenderLayers) {
    spawn_unit_frame(commands, &layer);
    spawn_origin_cross(commands, &layer);
}

/// [-1, 1] x [-1, 1] の単位正方形を囲む細い枠を生成する。
fn spawn_unit_frame(commands: &mut Commands, layer: &RenderLayers) {
    let color = Color::srgb(0.18, 0.18, 0.22);
    let thickness = 0.01;
    let sides = [
        (Vec2::new(2.0, thickness), Vec3::new(0.0, 1.0, 0.0)), // 上辺
        (Vec2::new(2.0, thickness), Vec3::new(0.0, -1.0, 0.0)), // 下辺
        (Vec2::new(thickness, 2.0), Vec3::new(-1.0, 0.0, 0.0)), // 左辺
        (Vec2::new(thickness, 2.0), Vec3::new(1.0, 0.0, 0.0)), // 右辺
    ];
    for (size, pos) in sides {
        commands.spawn((
            Sprite::from_color(color, size),
            Transform::from_translation(pos),
            layer.clone(),
        ));
    }
}

/// 原点 (0, 0) に小さな十字マーカーを生成する。
fn spawn_origin_cross(commands: &mut Commands, layer: &RenderLayers) {
    let color = Color::srgb(0.6, 0.6, 0.7);
    let arm = 0.05;
    let thickness = 0.005;
    commands.spawn((
        Sprite::from_color(color, Vec2::new(arm * 2.0, thickness)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        layer.clone(),
    ));
    commands.spawn((
        Sprite::from_color(color, Vec2::new(thickness, arm * 2.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        layer.clone(),
    ));
}
