//! 右側のパラメータパネル（egui）と、それに合わせた Edit/Result カメラの
//! ビューポート分配を提供するモジュール。

use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::fractal::replica_color;
use crate::state::{FractalState, Replica, UndoStack};
use crate::{EditCamera, ResultCamera};

/// パラメータパネルとビューポート更新を登録するプラグイン。
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, params_panel);
    }
}

/// 右ドックにパラメータパネルを描画し、その幅から残った領域を
/// Edit / Result カメラに左右半々で割り当てる。
fn params_panel(
    mut contexts: EguiContexts,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FractalState>,
    mut undo_stack: ResMut<UndoStack>,
    mut edit_cam: Query<&mut Camera, (With<EditCamera>, Without<ResultCamera>)>,
    mut result_cam: Query<&mut Camera, (With<ResultCamera>, Without<EditCamera>)>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let panel_logical_w = egui::SidePanel::right("params")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            draw_top_controls(ui, &mut state, &mut undo_stack);
            draw_replica_section(ui, &mut state, &mut undo_stack);
        })
        .response
        .rect
        .width();

    apply_canvas_viewports(panel_logical_w, &windows, &mut edit_cam, &mut result_cam);

    Ok(())
}

/// パネル上部：見出し・Depth スライダ・線のクリア・複製の追加を表示する。
fn draw_top_controls(ui: &mut egui::Ui, state: &mut FractalState, undo_stack: &mut UndoStack) {
    ui.heading("Parameters");
    ui.separator();
    ui.add(egui::Slider::new(&mut state.depth, 1..=12).text("Depth"));
    ui.separator();

    ui.label(format!("Lines: {}", state.base_shape.lines.len()));
    if ui.button("Clear lines").clicked() {
        undo_stack.push(state.clone());
        state.base_shape.lines.clear();
    }
    ui.separator();

    ui.label(format!("Replicas: {}", state.replicas.len()));
    if ui.button("+ Add replica").clicked() {
        undo_stack.push(state.clone());
        state.replicas.push(Replica::default_new());
    }
    ui.separator();
}

/// 各 replica の編集 UI をリスト表示する。Delete ボタンが押された複製を 1 件削除する。
fn draw_replica_section(ui: &mut egui::Ui, state: &mut FractalState, undo_stack: &mut UndoStack) {
    let mut to_delete: Option<usize> = None;
    for (i, replica) in state.replicas.iter_mut().enumerate() {
        if draw_replica_editor(ui, i, replica) {
            to_delete = Some(i);
        }
    }
    if let Some(i) = to_delete {
        undo_stack.push(state.clone());
        state.replicas.remove(i);
    }
}

/// 1 つの複製を編集する折りたたみ UI。Delete が押されたら `true` を返す。
fn draw_replica_editor(ui: &mut egui::Ui, i: usize, replica: &mut Replica) -> bool {
    let title = egui::RichText::new(format!("Replica {i}"))
        .color(replica_title_color(i))
        .strong();
    let mut delete_requested = false;
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            translation_row(ui, &mut replica.translation);
            rotation_row(ui, &mut replica.rotation);
            scale_row(ui, &mut replica.scale);
            if ui.button("Delete this replica").clicked() {
                delete_requested = true;
            }
        });
    delete_requested
}

/// `replica_color`（リニア RGB）を sRGB に近似変換して egui の色に揃える。
fn replica_title_color(i: usize) -> egui::Color32 {
    let lin = replica_color(i);
    egui::Color32::from_rgb(
        (lin.red.powf(1.0 / 2.2) * 255.0) as u8,
        (lin.green.powf(1.0 / 2.2) * 255.0) as u8,
        (lin.blue.powf(1.0 / 2.2) * 255.0) as u8,
    )
}

/// 平行移動 (TX, TY) の編集行。
fn translation_row(ui: &mut egui::Ui, translation: &mut Vec2) {
    ui.horizontal(|ui| {
        ui.label("TX");
        ui.add(
            egui::DragValue::new(&mut translation.x)
                .speed(0.01)
                .range(-2.0..=2.0),
        );
        ui.label("TY");
        ui.add(
            egui::DragValue::new(&mut translation.y)
                .speed(0.01)
                .range(-2.0..=2.0),
        );
    });
}

/// 回転（度数で表示）の編集行。内部値はラジアンなので度数に変換して扱う。
fn rotation_row(ui: &mut egui::Ui, rotation: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Rot (deg)");
        let mut deg = rotation.to_degrees();
        if ui
            .add(
                egui::DragValue::new(&mut deg)
                    .speed(0.5)
                    .range(-180.0..=180.0),
            )
            .changed()
        {
            *rotation = deg.to_radians();
        }
    });
}

/// 一様スケールの編集行。
fn scale_row(ui: &mut egui::Ui, scale: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Scale");
        ui.add(egui::DragValue::new(scale).speed(0.005).range(0.05..=2.0));
    });
}

/// 計算したビューポートを Edit / Result カメラに書き込む。
fn apply_canvas_viewports(
    panel_logical_w: f32,
    windows: &Query<&Window, With<PrimaryWindow>>,
    edit_cam: &mut Query<&mut Camera, (With<EditCamera>, Without<ResultCamera>)>,
    result_cam: &mut Query<&mut Camera, (With<ResultCamera>, Without<EditCamera>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some((edit_vp, result_vp)) = compute_canvas_viewports(panel_logical_w, window) else {
        return;
    };
    if let Ok(mut cam) = edit_cam.single_mut() {
        cam.viewport = Some(edit_vp);
    }
    if let Ok(mut cam) = result_cam.single_mut() {
        cam.viewport = Some(result_vp);
    }
}

/// パネル幅（論理ピクセル）と現在のウィンドウサイズから、
/// Edit と Result の物理ピクセルビューポートを左右半々で算出する。
/// パネルがウィンドウより広いときなど描画不能な状況では `None`。
fn compute_canvas_viewports(panel_logical_w: f32, window: &Window) -> Option<(Viewport, Viewport)> {
    let scale = window.scale_factor();
    let panel_phys = (panel_logical_w * scale).round() as u32;
    let win_w = window.physical_width();
    let win_h = window.physical_height();
    if win_w <= panel_phys || win_h == 0 {
        return None;
    }
    let canvas_w = win_w - panel_phys;
    let half = canvas_w / 2;
    let edit_vp = Viewport {
        physical_position: UVec2::ZERO,
        physical_size: UVec2::new(half.max(1), win_h),
        ..default()
    };
    let result_vp = Viewport {
        physical_position: UVec2::new(half, 0),
        physical_size: UVec2::new((canvas_w - half).max(1), win_h),
        ..default()
    };
    Some((edit_vp, result_vp))
}
