//! 起動時のプラグイン登録順・初期リソース・Startup システムを束ねる。
//!
//! ウィンドウ設定などターゲットごとの差は `main` に残し、Bevy の `App` 組み立ての本体はここに集約する。

use bevy::camera::{OrthographicProjection, Projection, ScalingMode, visibility::RenderLayers};
use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, EguiPlugin, PrimaryEguiContext};

use crate::app::export::{ResultExportPlugin, ResultImageOutlet};
use crate::app::mode_state::AppScreenPlugin;
use crate::app::mode_state::startup::BootstrapUsedPresetEnv;
use crate::app::platform_handles::PlatformHandles;
use crate::app::session::{CanvasLayout, FractalState, SnapGrid, UiLayout};
use crate::app::session_rules::clamp_fractal_state_depth;
use crate::app::share::{ShareNavigation, SharePlugin};
use crate::fractal_presets::FractalPreset;
use crate::platform;
use crate::ui::UiPlugin;
use crate::ui::canvas::placement::PlacementPlugin;
use crate::ui::canvas::result::navigation::ViewPlugin;
use crate::ui::canvas::result::scene::FractalPlugin;
use crate::ui::canvas::seed::EditPlugin;

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

/// ネイティブ・WASM 共通の `App` を組み立てて実行する。
pub fn run() {
    let (initial_fractal, used_boot_preset) = initial_fractal_state();

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
            AppScreenPlugin,
            ResultExportPlugin,
            UiPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.10)))
        .insert_resource(PlatformHandles {
            share_navigation: ShareNavigation(platform::share_navigation_arc()),
            result_png_outlet: ResultImageOutlet(platform::png_export_sink_arc()),
        })
        .insert_resource(initial_fractal)
        .insert_resource(BootstrapUsedPresetEnv(used_boot_preset))
        .insert_resource(CanvasLayout::default())
        .insert_resource(SnapGrid::default())
        .insert_resource(UiLayout::default())
        .add_systems(Startup, setup)
        .run();
}

/// 環境変数プリセットがあればそれから、なければデフォルトの `FractalState` を構築する。
///
/// # 戻り値
/// 初期状態と、`FRACTALIUM_BOOT_PRESET` で認識可能な名前が載っていたか（ネイティブの画面上位モード判定用）。
fn initial_fractal_state() -> (FractalState, bool) {
    let mut used_boot = false;
    let mut s = if let Ok(raw) = std::env::var("FRACTALIUM_BOOT_PRESET") {
        if let Some(preset) = FractalPreset::from_boot_token(&raw) {
            used_boot = true;
            preset.build()
        } else {
            FractalState::default()
        }
    } else {
        FractalState::default()
    };
    clamp_fractal_state_depth(&mut s);
    (s, used_boot)
}

/// `index.html` のローディングオーバーレイを非表示にし、アクセシビリティ属性を更新する。
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

/// Edit / Placement 用の `AutoMin`。Result は `ui::canvas::result::navigation` の `RESULT_ORTHO_AUTOMIN`（2.0）が前提のため、ここでは広げない。
const EDIT_PLACEMENT_ORTHO_AUTOMIN: f32 = 2.5;

/// 単位正方形 [-1, 1]² が常に画面内に収まる正規化直交投影を返す（Result カメラ用。`RESULT_ORTHO_AUTOMIN` と同じ 2.0）。
fn normalized_projection() -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::AutoMin {
            min_width: 2.0,
            min_height: 2.0,
        },
        ..OrthographicProjection::default_2d()
    })
}

/// Edit / Placement カメラ用。`normalized_projection` より視野をわずかに広げ、枠外配置のレプリカにも余白を確保する。
fn edit_placement_normalized_projection() -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::AutoMin {
            min_width: EDIT_PLACEMENT_ORTHO_AUTOMIN,
            min_height: EDIT_PLACEMENT_ORTHO_AUTOMIN,
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
        edit_placement_normalized_projection(),
        edit_layer(),
    ));
    commands.spawn((
        PlacementCamera,
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        edit_placement_normalized_projection(),
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

/// `commands` に、指定 `layer` 上で [-1, 1]² を囲む細い枠スプライトを生成する。
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

/// `commands` に、指定 `layer` 上で原点付近の十字マーカー用スプライトを生成する。
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
