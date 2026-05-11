//! 短寿命の入力ガード用フラグ（ダブルタップズーム進行中など）。

use bevy::prelude::*;

/// ダブルタップドラッグによるズームが進行中かどうかを示すフラグ。
/// edit / placement システムがタッチ入力を無視するために参照する。
#[derive(Resource, Default)]
pub struct DoubleTapZoomActive(pub bool);
