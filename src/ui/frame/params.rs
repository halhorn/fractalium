//! 右ドックまたはナロー下部の Parameters: レプリカ一覧とスライダー群。

use bevy::prelude::Vec2;
use bevy_egui::egui;

use crate::app::session::{FractalState, UiLayout, UndoStack};
use crate::core::shape::{REPLICA_SCALE_MAX, REPLICA_SCALE_MIN, Replica};
use crate::ui::canvas::result::scene::result_replica_color;

/// Parameters の `DragValue` に表示する有効数字の桁数。
const PARAM_DRAG_VALUE_SIG_FIGS: usize = 6;

/// 折りたたみ状態を含むパラメータパネル全体（wide: 右ドック、narrow: ヘッダのみの切り替えを含む）。
pub(crate) fn draw_params_panel(
    ui: &mut egui::Ui,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
    ui_layout: &mut UiLayout,
) {
    if ui_layout.params_collapsed {
        if ui.button("▶").clicked() {
            ui_layout.params_collapsed = false;
        }
    } else {
        ui.horizontal(|ui| {
            if ui.button("◀").clicked() {
                ui_layout.params_collapsed = true;
            }
            ui.heading("Parameters");
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                draw_params_controls(ui, state, undo_stack);
            });
    }
}

/// パネル本体の統計ラベルとレプリカセクション。
pub(crate) fn draw_params_controls(
    ui: &mut egui::Ui,
    state: &mut FractalState,
    undo_stack: &mut UndoStack,
) {
    ui.label(format!("Lines: {}", state.base_shape.lines.len()));
    ui.add_space(6.0);

    ui.label(format!("Replicas: {}", state.replicas.len()));
    ui.add_space(6.0);

    draw_replica_section(ui, state, undo_stack);
}

/// 各行の折りたたみヘッダと削除ボタン。削除が選ばれたインデックスはここで除く。
fn draw_replica_section(ui: &mut egui::Ui, state: &mut FractalState, undo_stack: &mut UndoStack) {
    let total = state.replicas.len();
    let mut to_delete: Option<usize> = None;
    for (i, replica) in state.replicas.iter_mut().enumerate() {
        if draw_replica_editor(ui, i, total, replica) {
            to_delete = Some(i);
        }
    }
    if let Some(i) = to_delete {
        undo_stack.push(state.clone());
        state.replicas.remove(i);
    }
}

/// 単一レプリカの位置・回転・スケール編集。削除要求時は true。
fn draw_replica_editor(ui: &mut egui::Ui, i: usize, total: usize, replica: &mut Replica) -> bool {
    let title = egui::RichText::new(format!("Replica {i}"))
        .color(replica_title_color(i, total))
        .strong();
    let mut delete_requested = false;
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            position_row(ui, &mut replica.position);
            rotation_row(ui, &mut replica.rotation);
            scale_row(ui, &mut replica.scale);
            if ui.button("Delete this replica").clicked() {
                delete_requested = true;
            }
        });
    delete_requested
}

/// Result ビューと同系の線形色を、sRGB approx で egui 用に変換する。
fn replica_title_color(i: usize, total: usize) -> egui::Color32 {
    let lin = result_replica_color(i, total);
    egui::Color32::from_rgb(
        (lin.red.powf(1.0 / 2.2) * 255.0) as u8,
        (lin.green.powf(1.0 / 2.2) * 255.0) as u8,
        (lin.blue.powf(1.0 / 2.2) * 255.0) as u8,
    )
}

/// レプリカの平行移動 X/Y（正規化座標）。
fn position_row(ui: &mut egui::Ui, position: &mut Vec2) {
    ui.horizontal(|ui| {
        ui.label("X");
        ui.add(
            params_drag_value(&mut position.x)
                .speed(0.01)
                .range(-2.0..=2.0),
        );
        ui.label("Y");
        ui.add(
            params_drag_value(&mut position.y)
                .speed(0.01)
                .range(-2.0..=2.0),
        );
    });
}

/// レプリカの回転（度入力・内部はラジアン）。
fn rotation_row(ui: &mut egui::Ui, rotation: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Rotation (deg)");
        let mut deg = rotation.to_degrees();
        if ui
            .add(params_drag_value(&mut deg).speed(0.5).range(-180.0..=180.0))
            .changed()
        {
            *rotation = deg.to_radians();
        }
    });
}

/// レプリカのスケール（許容レンジは状態定数に従う）。
fn scale_row(ui: &mut egui::Ui, scale: &mut f32) {
    ui.horizontal(|ui| {
        ui.label("Scale");
        ui.add(
            params_drag_value(scale)
                .speed(0.005)
                .range(REPLICA_SCALE_MIN..=REPLICA_SCALE_MAX),
        );
    });
}

/// `n` を有効数字 [`PARAM_DRAG_VALUE_SIG_FIGS`] 桁で表示用に整形する。
///
/// # 引数
/// - `n`: 表示する値。
///
/// # 戻り値
/// 整形済みの文字列（非有限は `to_string` に委ねる）。
fn format_param_drag_value(n: f64) -> String {
    if !n.is_finite() {
        return n.to_string();
    }
    if n == 0.0 {
        return format!("{:.6}", 0.0_f64);
    }
    let exp = n.abs().log10().floor() as i32;
    let dp = (PARAM_DRAG_VALUE_SIG_FIGS as i32 - 1 - exp).clamp(0, 15) as usize;
    format!("{:.*}", dp, n)
}

/// Parameters の数値入力を `f64` に戻す。
///
/// # 引数
/// - `s`: ユーザーが編集したテキスト。
///
/// # 戻り値
/// パースできた場合はその値。失敗時は `None`。
fn parse_param_drag_value(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}

/// Parameters で共通する、有効数字付き [`egui::DragValue`]。
///
/// # 引数
/// - `value`: バインドする `f32` への可変参照。
///
/// # 戻り値
/// `.speed` / `.range` などを続けて表示用に調整できる [`egui::DragValue`]。
fn params_drag_value<'a>(value: &'a mut f32) -> egui::DragValue<'a> {
    egui::DragValue::new(value)
        .custom_formatter(|n, _| format_param_drag_value(n))
        .custom_parser(parse_param_drag_value)
}
