use std::f32::consts::FRAC_PI_4;

use bevy::prelude::Vec2;

use crate::fractal_presets::common::{flip_y, rotate_neg_90, state};
use crate::seed_shape::BaseShapePreset;
use crate::state::{FractalState, Line, Replica};

/// 種線分の中点から ±45° に二分枝する対称木。全体を -90° 回して幹が +y（上向き）になる。
/// 各写像は中点を固定点にするよう並進を付ける。
pub(super) fn build() -> FractalState {
    let s = 0.52_f32;
    let theta = FRAC_PI_4;
    let (sin_t, cos_t) = theta.sin_cos();
    let t0 = Vec2::new(s * cos_t, s * sin_t);
    let t1 = Vec2::new(s * cos_t, -s * sin_t);
    // x 軸鏡映と IFS の整合: 並進の y を反転し、回転角は符号反転。
    let replicas = vec![
        Replica {
            translation: flip_y(rotate_neg_90(t0)),
            rotation: -theta,
            scale: s,
        },
        Replica {
            translation: flip_y(rotate_neg_90(t1)),
            rotation: theta,
            scale: s,
        },
    ];
    let base: Vec<Line> = BaseShapePreset::Segment
        .lines()
        .into_iter()
        .map(|l| Line {
            a: flip_y(rotate_neg_90(l.b)),
            b: flip_y(rotate_neg_90(l.a)),
        })
        .collect();
    state(base, replicas, 11, true)
}
