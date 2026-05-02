//! グリッドスナップ計算と格子点描画を提供する。
//!
//! 通常グリッド（直交 8 等分 + 等角 6 等分）と
//! 細かいグリッド（直交 16 等分 + 等角 12 等分）の 2 種類を用意する。
//! 描画関数は GizmoConfigGroup を型パラメータにとるため、
//! Edit / Placement どちらのギズモグループでも使用できる。

use bevy::prelude::*;

const GRID_SQUARE_DIV: f32 = 8.0;
const GRID_TRI_DIV: f32 = 6.0;
const FINE_SQUARE_DIV: f32 = 16.0;
const FINE_TRI_DIV: f32 = 12.0;
/// グリッドの描画・スナップ範囲（[-GRID_RANGE, GRID_RANGE]）。
const GRID_RANGE: f32 = 3.0;

const GRID_DOT_RADIUS: f32 = 0.012;
const GRID_HIGHLIGHT_RADIUS: f32 = 0.025;
const GRID_DOT_COLOR: Color = Color::srgba(0.6, 0.7, 1.0, 0.35);
const GRID_HIGHLIGHT_COLOR: Color = Color::srgba(0.4, 1.0, 0.6, 0.9);

// === スナップ計算 ===

/// 直交格子と等角格子のうち、`pos` により近い格子点を返す（通常解像度）。
pub fn snap_to_grid(pos: Vec2) -> Vec2 {
    let sq = nearest_square_point(pos, GRID_SQUARE_DIV);
    let iso = nearest_iso_point(pos, GRID_TRI_DIV);
    if iso.distance_squared(pos) <= sq.distance_squared(pos) { iso } else { sq }
}

/// 直交格子と等角格子のうち、`pos` により近い格子点を返す（高解像度）。
pub fn snap_to_fine_grid(pos: Vec2) -> Vec2 {
    let sq = nearest_square_point(pos, FINE_SQUARE_DIV);
    let iso = nearest_iso_point(pos, FINE_TRI_DIV);
    if iso.distance_squared(pos) <= sq.distance_squared(pos) { iso } else { sq }
}

fn nearest_square_point(pos: Vec2, div: f32) -> Vec2 {
    let s = 2.0 / div;
    Vec2::new((pos.x / s).round() * s, (pos.y / s).round() * s)
}

fn nearest_iso_point(pos: Vec2, div: f32) -> Vec2 {
    let h = 2.0 / div;
    let v = h * (3.0_f32.sqrt() / 2.0);
    let m_center = ((pos.y + GRID_RANGE) / v).round() as i32;
    let mut best = Vec2::ZERO;
    let mut best_dist = f32::MAX;
    for m in (m_center - 1)..=(m_center + 1) {
        let row_y = -GRID_RANGE + m as f32 * v;
        let offset = if m.rem_euclid(2) == 0 { 0.0 } else { h / 2.0 };
        let n_center = ((pos.x + GRID_RANGE - offset) / h).round() as i32;
        for n in (n_center - 1)..=(n_center + 1) {
            let candidate = Vec2::new(-GRID_RANGE + n as f32 * h + offset, row_y);
            let dist = candidate.distance_squared(pos);
            if dist < best_dist {
                best_dist = dist;
                best = candidate;
            }
        }
    }
    best
}

// === 格子点描画（GizmoConfigGroup に対してジェネリック） ===

/// 通常解像度グリッドを描画する（直交 8 等分 + 等角 6 等分）。
pub fn draw_grid<G: GizmoConfigGroup>(gizmos: &mut Gizmos<G>, highlighted: Option<Vec2>) {
    draw_square_grid(gizmos, GRID_SQUARE_DIV, highlighted);
    draw_iso_grid(gizmos, GRID_TRI_DIV, GRID_SQUARE_DIV, highlighted);
}

/// 細かいグリッドを描画する（直交 16 等分 + 等角 12 等分）。
pub fn draw_fine_grid<G: GizmoConfigGroup>(gizmos: &mut Gizmos<G>, highlighted: Option<Vec2>) {
    draw_square_grid(gizmos, FINE_SQUARE_DIV, highlighted);
    draw_iso_grid(gizmos, FINE_TRI_DIV, FINE_SQUARE_DIV, highlighted);
}

fn draw_dot<G: GizmoConfigGroup>(gizmos: &mut Gizmos<G>, pt: Vec2, highlighted: Option<Vec2>) {
    if highlighted.is_some_and(|h| h.distance_squared(pt) < 1e-5) {
        gizmos.circle_2d(pt, GRID_HIGHLIGHT_RADIUS, GRID_HIGHLIGHT_COLOR);
    } else {
        gizmos.circle_2d(pt, GRID_DOT_RADIUS, GRID_DOT_COLOR);
    }
}

fn draw_square_grid<G: GizmoConfigGroup>(gizmos: &mut Gizmos<G>, div: f32, highlighted: Option<Vec2>) {
    let s = 2.0 / div;
    let n = (2.0 * GRID_RANGE / s).round() as i32;
    for xi in 0..=n {
        for yi in 0..=n {
            draw_dot(gizmos, Vec2::new(-GRID_RANGE + xi as f32 * s, -GRID_RANGE + yi as f32 * s), highlighted);
        }
    }
}

/// 等角格子を描画する。直交格子と重なる点はスキップする。
fn draw_iso_grid<G: GizmoConfigGroup>(
    gizmos: &mut Gizmos<G>,
    tri_div: f32,
    sq_div: f32,
    highlighted: Option<Vec2>,
) {
    let h = 2.0 / tri_div;
    let v = h * (3.0_f32.sqrt() / 2.0);
    let sq_s = 2.0 / sq_div;
    let m_max = (2.0 * GRID_RANGE / v).ceil() as i32;
    for m in 0..=m_max {
        let y = -GRID_RANGE + m as f32 * v;
        if y > GRID_RANGE + 0.01 { break; }
        let offset = if m.rem_euclid(2) == 0 { 0.0 } else { h / 2.0 };
        let n_max = ((2.0 * GRID_RANGE - offset) / h).ceil() as i32;
        for n in 0..=n_max {
            let x = -GRID_RANGE + n as f32 * h + offset;
            if x > GRID_RANGE + 0.01 { continue; }
            let pt = Vec2::new(x, y);
            let on_sq = (pt.x / sq_s).fract().abs() < 1e-4 && (pt.y / sq_s).fract().abs() < 1e-4;
            if !on_sq {
                draw_dot(gizmos, pt, highlighted);
            }
        }
    }
}
