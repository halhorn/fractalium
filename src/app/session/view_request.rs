//! 一度処理したら消えるビュー意図（カメラフィット要求など）。

use bevy::prelude::*;

/// URL からの復元やフルプリセット適用後、Result カメラを図形に合わせる。
#[derive(Resource, Default)]
pub struct PendingResultCameraFit(pub bool);
