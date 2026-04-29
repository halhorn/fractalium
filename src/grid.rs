//! Edit キャンバスのグリッドスナップ計算と、グリッドドットの描画を提供する。
//!
//! 直交 8 等分グリッド（四角形を描きやすい）と、等角 6 等分グリッド（正三角形を
//! 描きやすい）の 2 種類を重ね合わせて表示し、両者のうち近い方の格子点に
//! スナップする。

use bevy::prelude::*;

use crate::edit::EditGizmos;

/// 直交格子の分割数（[-1, 1] の幅 2 を 8 分割）。
const GRID_SQUARE_DIV: f32 = 8.0;
/// 等角格子の水平分割数（[-1, 1] の幅 2 を 6 分割）。
const GRID_TRI_DIV: f32 = 6.0;

const GRID_DOT_RADIUS: f32 = 0.012;
const GRID_HIGHLIGHT_RADIUS: f32 = 0.025;
const GRID_DOT_COLOR: Color = Color::srgba(0.6, 0.7, 1.0, 0.35);
const GRID_HIGHLIGHT_COLOR: Color = Color::srgba(0.4, 1.0, 0.6, 0.9);

/// 直交格子と等角格子のうち、`pos` により近い格子点を返す。
pub fn snap_to_grid(pos: Vec2) -> Vec2 {
    let sq = nearest_square_grid_point(pos);
    let iso = nearest_iso_grid_point(pos);
    if iso.distance_squared(pos) <= sq.distance_squared(pos) {
        iso
    } else {
        sq
    }
}

/// 直交 8 等分グリッドの最近傍格子点を返す。
fn nearest_square_grid_point(pos: Vec2) -> Vec2 {
    let s = 2.0 / GRID_SQUARE_DIV;
    Vec2::new((pos.x / s).round() * s, (pos.y / s).round() * s)
}

/// 等角（三角格子）6 等分グリッドの最近傍格子点を返す。
/// 行ごとに半周期ずれる三角格子のため、近傍行・列を 3x3 で探索して最近接を採る。
fn nearest_iso_grid_point(pos: Vec2) -> Vec2 {
    let h = 2.0 / GRID_TRI_DIV;
    let v = h * (3.0_f32.sqrt() / 2.0);
    let m_center = ((pos.y + 1.0) / v).round() as i32;

    let mut best = Vec2::ZERO;
    let mut best_dist = f32::MAX;
    for m in (m_center - 1)..=(m_center + 1) {
        let row_y = -1.0 + m as f32 * v;
        let offset = if m.rem_euclid(2) == 0 { 0.0 } else { h / 2.0 };
        let n_center = ((pos.x + 1.0 - offset) / h).round() as i32;
        for n in (n_center - 1)..=(n_center + 1) {
            let candidate = Vec2::new(-1.0 + n as f32 * h + offset, row_y);
            let dist = candidate.distance_squared(pos);
            if dist < best_dist {
                best_dist = dist;
                best = candidate;
            }
        }
    }
    best
}

/// Edit キャンバスへ直交＋等角グリッドのドットを描画する。
/// `highlighted` が指定されたら、その格子点だけ強調色で描く。
pub fn draw_grid(gizmos: &mut Gizmos<EditGizmos>, highlighted: Option<Vec2>) {
    draw_square_grid(gizmos, highlighted);
    draw_iso_grid(gizmos, highlighted);
}

/// 1 つの格子点を、ハイライト対象かどうかで色とサイズを切り替えて描画する。
fn draw_dot(gizmos: &mut Gizmos<EditGizmos>, pt: Vec2, highlighted: Option<Vec2>) {
    let is_highlight = highlighted.is_some_and(|h| h.distance_squared(pt) < 1e-5);
    if is_highlight {
        gizmos.circle_2d(pt, GRID_HIGHLIGHT_RADIUS, GRID_HIGHLIGHT_COLOR);
    } else {
        gizmos.circle_2d(pt, GRID_DOT_RADIUS, GRID_DOT_COLOR);
    }
}

/// 直交 8 等分グリッドの格子点を全て描画する。
fn draw_square_grid(gizmos: &mut Gizmos<EditGizmos>, highlighted: Option<Vec2>) {
    let s = 2.0 / GRID_SQUARE_DIV;
    let n = GRID_SQUARE_DIV as i32;
    for xi in 0..=n {
        for yi in 0..=n {
            let pt = Vec2::new(-1.0 + xi as f32 * s, -1.0 + yi as f32 * s);
            draw_dot(gizmos, pt, highlighted);
        }
    }
}

/// 等角 6 等分グリッドの格子点を描画する。直交格子とぴったり重なる点は二重描画を避けてスキップする。
fn draw_iso_grid(gizmos: &mut Gizmos<EditGizmos>, highlighted: Option<Vec2>) {
    let h = 2.0 / GRID_TRI_DIV;
    let v = h * (3.0_f32.sqrt() / 2.0);
    let sq_s = 2.0 / GRID_SQUARE_DIV;
    let m_max = (2.0 / v).ceil() as i32;
    for m in 0..=m_max {
        let y = -1.0 + m as f32 * v;
        if y > 1.01 {
            break;
        }
        let offset = if m.rem_euclid(2) == 0 { 0.0 } else { h / 2.0 };
        let n_max = ((2.0 - offset) / h).ceil() as i32;
        for n in 0..=n_max {
            let x = -1.0 + n as f32 * h + offset;
            if x > 1.01 {
                continue;
            }
            let pt = Vec2::new(x, y);
            let on_sq_grid =
                (pt.x / sq_s).fract().abs() < 1e-4 && (pt.y / sq_s).fract().abs() < 1e-4;
            if on_sq_grid {
                continue;
            }
            draw_dot(gizmos, pt, highlighted);
        }
    }
}
