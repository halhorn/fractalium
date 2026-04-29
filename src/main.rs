use bevy::camera::{Viewport, visibility::RenderLayers};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext, egui,
};

#[derive(Component)]
struct EditCamera;

#[derive(Component)]
struct ResultCamera;

// Spatial offset so each camera only sees its own area's content
// (cheap separation without RenderLayers for the MVP skeleton).
const RESULT_AREA_OFFSET_X: f32 = 10_000.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Fractalium".to_string(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.10)))
        .add_systems(Startup, setup)
        .add_systems(EguiPrimaryContextPass, params_panel)
        .run();
}

fn setup(mut commands: Commands, mut egui_global: ResMut<EguiGlobalSettings>) {
    // Disable auto-create so we can attach `PrimaryEguiContext` to a dedicated
    // full-window UI camera. Without this, egui anchors to one of our viewport-
    // limited world cameras and SidePanel positions break.
    egui_global.auto_create_primary_context = false;

    commands.spawn((
        EditCamera,
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        ResultCamera,
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        Transform::from_xyz(RESULT_AREA_OFFSET_X, 0.0, 0.0),
    ));

    // Dedicated egui camera: covers the full window, renders no world content,
    // overlays UI on top of the world cameras' output.
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

    let label_color = TextColor(Color::srgb(0.7, 0.7, 0.8));
    let label_font = TextFont {
        font_size: 48.0,
        ..default()
    };

    commands.spawn((
        Text2d::new("Edit"),
        label_font.clone(),
        label_color,
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        Text2d::new("Result"),
        label_font,
        label_color,
        Transform::from_xyz(RESULT_AREA_OFFSET_X, 0.0, 0.0),
    ));
}

fn params_panel(
    mut contexts: EguiContexts,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut edit_cam: Query<&mut Camera, (With<EditCamera>, Without<ResultCamera>)>,
    mut result_cam: Query<&mut Camera, (With<ResultCamera>, Without<EditCamera>)>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let panel_logical_w = egui::SidePanel::right("params")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Parameters");
            ui.separator();
            ui.label("(F2 skeleton — controls land here in F4 / Features)");
        })
        .response
        .rect
        .width();

    let Ok(window) = windows.single() else {
        return Ok(());
    };
    let scale = window.scale_factor();
    let panel_phys = (panel_logical_w * scale).round() as u32;
    let win_w = window.physical_width();
    let win_h = window.physical_height();
    if win_w <= panel_phys || win_h == 0 {
        return Ok(());
    }
    let canvas_w = win_w - panel_phys;
    let half = canvas_w / 2;

    if let Ok(mut cam) = edit_cam.single_mut() {
        cam.viewport = Some(Viewport {
            physical_position: UVec2::ZERO,
            physical_size: UVec2::new(half.max(1), win_h),
            ..default()
        });
    }
    if let Ok(mut cam) = result_cam.single_mut() {
        cam.viewport = Some(Viewport {
            physical_position: UVec2::new(half, 0),
            physical_size: UVec2::new((canvas_w - half).max(1), win_h),
            ..default()
        });
    }

    Ok(())
}
