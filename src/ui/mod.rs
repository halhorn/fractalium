//! 右側のパラメータパネル（egui）と左側ペイン（Seed / Placement）の描画、
//! および各カメラへのビューポート分配を提供するモジュール。
//!
//! **フレーム**（パネル・タイトル帯）は [`frame`]。**ワールドキャンバス**（シード／配置／結果と格子）は [`canvas`]。
//! [`viewport_bridge`] は egui 矩形から [`bevy::camera::Camera`] のビューポートへ写すだけの橋渡し。

pub mod canvas;
pub mod feedback;
mod frame;
mod preset_picker;
mod viewport_bridge;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

use crate::app::export::PreparedResultImage;
use crate::app::mode_state::AppScreen;
use crate::app::platform_handles::PlatformHandles;
use crate::app::session::{
    CanvasLayout, FractalState, PendingResultCameraFit, PlacementState, SnapGrid, UiLayout,
    UndoStack,
};
use crate::ui::canvas::result::navigation::fit_result_camera_if_requested;
use crate::ui::canvas::seed::DrawState;
use crate::ui::feedback::toast::{DeferredToast, EguiToast};
use crate::ui::preset_picker::{
    PresetPickerNeedsInitialFocus, PresetPickerTileSelection, PresetThumbnailCache,
    paint_preset_picker_screen, reset_preset_picker_on_enter,
};

use self::frame::depth_controller::paint_depth_controls;
use self::frame::layout::{layout_narrow, layout_wide};
use self::viewport_bridge::{ViewportCamerasMut, egui_rect_to_viewport};

/// [`params_panel`] が毎フレーム触るフラクタル編集・レイアウト用リソースを束ね、システム引数上限に収める。
#[derive(SystemParam)]
struct FractalPanelUi<'w> {
    /// 共有・プリセットの芯となるフラクタル定義。
    fractal: ResMut<'w, FractalState>,
    /// シード／配置グリッドのスナップ。
    snap_grid: ResMut<'w, SnapGrid>,
    /// シード編集サブ状態。
    draw_state: ResMut<'w, DrawState>,
    /// Undo / Redo 履歴。
    undo_stack: ResMut<'w, UndoStack>,
    /// レプリカの選択とドラッグ。
    placement: ResMut<'w, PlacementState>,
    /// ヒットテスト用のキャンバス矩形キャッシュ。
    layout: ResMut<'w, CanvasLayout>,
    /// 折りたたみ付き UI レイアウト。
    ui_layout: ResMut<'w, UiLayout>,
}

/// プリセット画面と [`AppScreen`] 遷移に必要なリソース。
#[derive(SystemParam)]
struct PresetPickerPanelFlow<'w> {
    /// いまの画面上位モード。
    screen: Res<'w, State<AppScreen>>,
    /// 次フレームへ回す遷移意図。
    next: ResMut<'w, NextState<AppScreen>>,
    /// サムネイルの遅延ロードキャッシュ。
    thumbs: ResMut<'w, PresetThumbnailCache>,
    /// タイルの視覚選択インデックス。
    tile_sel: ResMut<'w, PresetPickerTileSelection>,
    /// 入場直後のフォーカス要求。
    focus_flag: ResMut<'w, PresetPickerNeedsInitialFocus>,
}

/// egui のメインコンテキストPassでパネルとビューポートを更新する [`Plugin`]。
pub struct UiPlugin;

impl Plugin for UiPlugin {
    /// トースト・カメラフィット・プリセット画面用リソースを初期化し、[`params_panel`] を `EguiPrimaryContextPass` に登録する。
    fn build(&self, app: &mut App) {
        app.init_resource::<DeferredToast>()
            .init_resource::<EguiToast>()
            .init_resource::<PendingResultCameraFit>()
            .init_resource::<PresetThumbnailCache>()
            .init_resource::<PresetPickerTileSelection>()
            .init_resource::<PresetPickerNeedsInitialFocus>()
            .add_systems(
                OnEnter(AppScreen::PresetPicker),
                reset_preset_picker_on_enter,
            );

        app.add_systems(EguiPrimaryContextPass, params_panel);

        app.add_systems(
            EguiPrimaryContextPass,
            fit_result_camera_if_requested.after(params_panel),
        );
    }
}

/// パネル配置・Result オーバーレイ・トーストの 1 フレーム分をまとめ、カメラ [`bevy::camera::Viewport`] を更新する。
fn params_panel(
    mut contexts: EguiContexts,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut fractal_panel: FractalPanelUi,
    mut toast: ResMut<EguiToast>,
    mut deferred_toast: ResMut<DeferredToast>,
    mut pending_result_fit: ResMut<PendingResultCameraFit>,
    mut prepared_png: ResMut<PreparedResultImage>,
    platform_handles: Res<PlatformHandles>,
    mut flow: PresetPickerPanelFlow,
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

    let current_screen = *flow.screen.get();

    let (edit_egui_rect, placement_egui_rect, result_egui_rect) = match current_screen {
        AppScreen::PresetPicker => {
            paint_preset_picker_screen(
                ctx,
                win_w,
                win_h,
                &mut fractal_panel.fractal,
                &mut fractal_panel.undo_stack,
                &mut fractal_panel.draw_state,
                &mut fractal_panel.placement,
                &mut pending_result_fit,
                &mut flow.next,
                &mut flow.thumbs,
                &mut flow.tile_sel,
                &mut flow.focus_flag,
            );
            let full_window =
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(win_w, win_h));
            (egui::Rect::NOTHING, egui::Rect::NOTHING, full_window)
        }
        AppScreen::Editing => {
            if is_narrow {
                layout_narrow(
                    ctx,
                    &mut commands,
                    win_w,
                    win_h,
                    &mut fractal_panel.fractal,
                    &mut fractal_panel.snap_grid,
                    &mut fractal_panel.draw_state,
                    &mut fractal_panel.undo_stack,
                    &mut fractal_panel.placement,
                    &mut fractal_panel.ui_layout,
                    &mut toast,
                    &mut deferred_toast,
                    &mut prepared_png,
                    share_nav,
                    png_outlet,
                    &mut flow.next,
                )
            } else {
                layout_wide(
                    ctx,
                    &mut commands,
                    win_w,
                    win_h,
                    &mut fractal_panel.fractal,
                    &mut fractal_panel.snap_grid,
                    &mut fractal_panel.draw_state,
                    &mut fractal_panel.undo_stack,
                    &mut fractal_panel.placement,
                    &mut fractal_panel.ui_layout,
                    &mut toast,
                    &mut deferred_toast,
                    &mut prepared_png,
                    share_nav,
                    png_outlet,
                    &mut flow.next,
                )
            }
        }
    };

    if matches!(current_screen, AppScreen::Editing) {
        paint_depth_controls(
            ctx,
            result_egui_rect,
            &mut fractal_panel.fractal,
            &mut fractal_panel.layout,
        );
    }
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

    fractal_panel.layout.placement_min_x = placement_egui_rect.min.x;
    fractal_panel.layout.placement_max_x = placement_egui_rect.max.x;
    fractal_panel.layout.placement_min_y = placement_egui_rect.min.y;
    fractal_panel.layout.placement_max_y = placement_egui_rect.max.y;

    Ok(())
}
