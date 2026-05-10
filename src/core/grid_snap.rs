//! ドラッグで置いた頂点やハンドルを、指やマウスでは難しい**きれいな位置**へそっと寄せる処理である。
//!
//! ユーザーには、細かな座標をいじらなくても対称や平行、きちんとした三角形や四角形といったレイアウトへ近づけられる効果がある。
//! また「だいたい同じような配置」を何度でも再現しやすくなる。Ctrl などで細かい格子へ寄せる切り替えも、同じ考えで段階を
//! 増やしている。
//!
//! 設計上は「ユーザーに伝わる挙動」はこの寄せ計算だけでほぼ決まる。そのため入力デバイスや描画とは切り離し、どの点が
//! **最終的に確定される座標になるか**だけをこのモジュールに閉じ込める。画面上のガイドラインやドットの見た目は `grid`
//! の責務で、ここには直交格子と三角格子の交点どちらが近いかで選んだ純計算のみを載せている。

use glam::Vec2;

// スナップ対象領域は `[-GRID_RANGE, GRID_RANGE]`²。直交格子の辺の分割数と三角格子の段数で解像度を決める。
const GRID_SQUARE_DIV: f32 = 8.0;
const GRID_TRI_DIV: f32 = 6.0;
const FINE_SQUARE_DIV: f32 = 16.0;
const FINE_TRI_DIV: f32 = 12.0;
const GRID_RANGE: f32 = 3.0;

/// 直交格子（8 等分）と三角格子（6 等分）の交点のうち、与えられた点に最も近い点へスナップする。
///
/// 編集キャンバスの通常モードで、ドラッグ確定時などに呼ぶ。
///
/// # Arguments
///
/// * `pos` - スナップ前の 2D 位置（正規化キャンバス座標）。
///
/// # Returns
///
/// 最寄りの格子点座標。
pub fn snap_to_grid(pos: Vec2) -> Vec2 {
    let sq = nearest_square_point(pos, GRID_SQUARE_DIV);
    let iso = nearest_iso_point(pos, GRID_TRI_DIV);
    if iso.distance_squared(pos) <= sq.distance_squared(pos) {
        iso
    } else {
        sq
    }
}

/// [`snap_to_grid`] より細かい直交・三角格子（16 / 12 等分）へスナップする。
///
/// Ctrl 押下時など高精度モード用。
///
/// # Arguments
///
/// * `pos` - スナップ前の 2D 位置（正規化キャンバス座標）。
///
/// # Returns
///
/// 高解像度格子の最寄り点。
pub fn snap_to_fine_grid(pos: Vec2) -> Vec2 {
    let sq = nearest_square_point(pos, FINE_SQUARE_DIV);
    let iso = nearest_iso_point(pos, FINE_TRI_DIV);
    if iso.distance_squared(pos) <= sq.distance_squared(pos) {
        iso
    } else {
        sq
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_origin_is_stable() {
        let o = Vec2::ZERO;
        assert_eq!(snap_to_grid(o), snap_to_grid(snap_to_grid(o)));
    }

    #[test]
    fn fine_snap_is_idempotent() {
        let p = Vec2::new(0.13, -0.07);
        let f = snap_to_fine_grid(p);
        assert_eq!(snap_to_fine_grid(f), f);
    }
}
