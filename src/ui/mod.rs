//! 右側のパラメータパネル（egui）と左側ペイン（Seed / Placement）の描画、
//! および各カメラへのビューポート分配を提供するモジュール。
//!
//! 幅 700px 以上: 左サイドパネル（Undo/Snap を含む操作バー + Edit + Placement）+ 中央 Result + 右パラメータパネル。
//! Result ウィンドウ左上に Depth（スライダー＋数値）と Show generations のトグルを重ね表示する。
//! 幅 700px 未満: 最上部タイトル + 中段 Result（上記オーバーレイを含む）
//!               + Undo/Snap グローバル操作バー + 下部 (Edit | Placement)
//!               + Parameters（折りたたみ／展開時は Result と干渉しない下部パネル）

mod global_bar;
mod layout;
mod params;
mod depth_controller;
mod seed_header;
mod shell;
mod viewport;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

use crate::edit::DrawState;
use crate::platform_handles::PlatformHandles;
use crate::result_export::PreparedResultImage;
use crate::state::{
    CanvasLayout, FractalState, PendingResultCameraFit, PlacementState, UiLayout, UndoStack,
};
use crate::toast::{DeferredToast, EguiToast};
use crate::view::fit_result_camera_if_requested;

use layout::{layout_narrow, layout_wide};
use depth_controller::paint_depth_controls;
use viewport::{ViewportCamerasMut, egui_rect_to_viewport};

/// egui のメインコンテキストPassでパネルとビューポートを更新する [`Plugin`]。
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// トースト・カメラフィット用リソースを初期化し、[`params_panel`] を `EguiPrimaryContextPass` に登録する。
    fn build(&self, app: &mut App) {
        app.init_resource::<DeferredToast>()
            .init_resource::<EguiToast>()
            .init_resource::<PendingResultCameraFit>();

        app.add_systems(EguiPrimaryContextPass, params_panel);

        app.add_systems(
            EguiPrimaryContextPass,
            fit_result_camera_if_requested.after(params_panel),
        );
    }
}

/// パネル配置・Result オーバーレイ・トーストの 1 フレーム分をまとめ、カメラ [`bevy::camera::Viewport`] を更新する。
#[allow(clippy::too_many_arguments)] // Bevy の `SystemParam` 1 システムぶんの引数。
fn params_panel(
    mut contexts: EguiContexts,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FractalState>,
    mut draw_state: ResMut<DrawState>,
    mut undo_stack: ResMut<UndoStack>,
    mut placement: ResMut<PlacementState>,
    mut layout: ResMut<CanvasLayout>,
    mut ui_layout: ResMut<UiLayout>,
    mut toast: ResMut<EguiToast>,
    mut deferred_toast: ResMut<DeferredToast>,
    mut pending_result_fit: ResMut<PendingResultCameraFit>,
    mut prepared_png: ResMut<PreparedResultImage>,
    platform_handles: Res<PlatformHandles>,
    mut cameras: ViewportCamerasMut,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let share_nav = &platform_handles.share_navigation;
    let png_outlet = &platform_handles.result_png_outlet;
    deferred_toast.flush_async_to_message();
    if let Some(msg) = std::mem::take(&mut deferred_toast.message) {
        toast.show(ctx, msg);
    }
    let Ok(window) = windows.single() else {
        return Ok(());
    };
    let scale = window.scale_factor();
    let win_w = window.width();
    let win_h = window.height();
    let win_phys = UVec2::new(window.physical_width(), window.physical_height());

    let is_narrow = win_w < 700.0;

    let (edit_egui_rect, placement_egui_rect, result_egui_rect) = if is_narrow {
        layout_narrow(
            ctx,
            &mut commands,
            win_w,
            win_h,
            &mut state,
            &mut draw_state,
            &mut undo_stack,
            &mut placement,
            &mut ui_layout,
            &mut toast,
            &mut deferred_toast,
            &mut prepared_png,
            share_nav,
            png_outlet,
            &mut pending_result_fit,
        )
    } else {
        layout_wide(
            ctx,
            &mut commands,
            win_w,
            win_h,
            &mut state,
            &mut draw_state,
            &mut undo_stack,
            &mut placement,
            &mut ui_layout,
            &mut toast,
            &mut deferred_toast,
            &mut prepared_png,
            share_nav,
            png_outlet,
            &mut pending_result_fit,
        )
    };

    paint_depth_controls(ctx, result_egui_rect, &mut state, &mut layout);
    toast.paint(ctx);
    if let Ok(mut cam) = cameras.edit_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(edit_egui_rect, scale, win_phys);
    }
    if let Ok(mut cam) = cameras.placement_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(placement_egui_rect, scale, win_phys);
    }
    if let Ok(mut cam) = cameras.result_cam.single_mut() {
        cam.viewport = egui_rect_to_viewport(result_egui_rect, scale, win_phys);
    }

    layout.placement_min_x = placement_egui_rect.min.x;
    layout.placement_max_x = placement_egui_rect.max.x;
    layout.placement_min_y = placement_egui_rect.min.y;
    layout.placement_max_y = placement_egui_rect.max.y;

    Ok(())
}
