mod draw;
mod state;

use bevy::camera::{
    OrthographicProjection, Projection, ScalingMode, Viewport, visibility::RenderLayers,
};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext, egui,
};
use draw::DrawPlugin;
use state::{FractalState, Replica};

#[derive(Component)]
pub struct EditCamera;

#[derive(Component)]
pub struct ResultCamera;

pub fn edit_layer() -> RenderLayers {
    RenderLayers::layer(1)
}
pub fn result_layer() -> RenderLayers {
    RenderLayers::layer(2)
}

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
        .add_plugins(DrawPlugin)
        .insert_resource(ClearColor(Color::srgb(0.08, 0.08, 0.10)))
        .insert_resource(FractalState::default_mvp())
        .add_systems(Startup, setup)
        .add_systems(EguiPrimaryContextPass, params_panel)
        .run();
}

fn normalized_projection() -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::AutoMin {
            min_width: 2.0,
            min_height: 2.0,
        },
        ..OrthographicProjection::default_2d()
    })
}

fn setup(mut commands: Commands, mut egui_global: ResMut<EguiGlobalSettings>) {
    egui_global.auto_create_primary_context = false;

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
        ResultCamera,
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        normalized_projection(),
        result_layer(),
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

    spawn_canvas_decor(&mut commands, edit_layer());
    spawn_canvas_decor(&mut commands, result_layer());
}

fn spawn_canvas_decor(commands: &mut Commands, layer: RenderLayers) {
    let frame_color = Color::srgb(0.45, 0.45, 0.55);
    let frame_thickness = 0.01;

    // Unit-square frame at [-1, 1] x [-1, 1].
    let sides = [
        // (size, position)
        (Vec2::new(2.0, frame_thickness), Vec3::new(0.0, 1.0, 0.0)), // top
        (Vec2::new(2.0, frame_thickness), Vec3::new(0.0, -1.0, 0.0)), // bottom
        (Vec2::new(frame_thickness, 2.0), Vec3::new(-1.0, 0.0, 0.0)), // left
        (Vec2::new(frame_thickness, 2.0), Vec3::new(1.0, 0.0, 0.0)), // right
    ];
    for (size, pos) in sides {
        commands.spawn((
            Sprite::from_color(frame_color, size),
            Transform::from_translation(pos),
            layer.clone(),
        ));
    }

    // Origin cross marker: small crosshair at (0, 0).
    let cross_color = Color::srgb(0.6, 0.6, 0.7);
    let cross_arm = 0.05;
    let cross_thickness = 0.005;
    commands.spawn((
        Sprite::from_color(cross_color, Vec2::new(cross_arm * 2.0, cross_thickness)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        layer.clone(),
    ));
    commands.spawn((
        Sprite::from_color(cross_color, Vec2::new(cross_thickness, cross_arm * 2.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        layer,
    ));
}

fn params_panel(
    mut contexts: EguiContexts,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FractalState>,
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
            ui.add(egui::Slider::new(&mut state.depth, 1..=7).text("Depth"));
            ui.separator();
            ui.label(format!("Lines: {}", state.base_shape.lines.len()));
            if ui.button("Clear lines").clicked() {
                state.base_shape.lines.clear();
            }
            ui.separator();
            ui.label(format!("Replicas: {}", state.replicas.len()));
            if ui.button("+ Add replica").clicked() {
                state.replicas.push(Replica::default_new());
            }
            ui.separator();

            let mut to_delete: Option<usize> = None;
            for (i, replica) in state.replicas.iter_mut().enumerate() {
                egui::CollapsingHeader::new(format!("Replica {i}"))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("TX");
                            ui.add(
                                egui::DragValue::new(&mut replica.translation.x)
                                    .speed(0.01)
                                    .range(-2.0..=2.0),
                            );
                            ui.label("TY");
                            ui.add(
                                egui::DragValue::new(&mut replica.translation.y)
                                    .speed(0.01)
                                    .range(-2.0..=2.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Rot (deg)");
                            let mut deg = replica.rotation.to_degrees();
                            if ui
                                .add(
                                    egui::DragValue::new(&mut deg)
                                        .speed(0.5)
                                        .range(-180.0..=180.0),
                                )
                                .changed()
                            {
                                replica.rotation = deg.to_radians();
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Scale");
                            ui.add(
                                egui::DragValue::new(&mut replica.scale)
                                    .speed(0.005)
                                    .range(0.05..=2.0),
                            );
                        });
                        if ui.button("Delete this replica").clicked() {
                            to_delete = Some(i);
                        }
                    });
            }
            if let Some(i) = to_delete {
                state.replicas.remove(i);
            }
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
