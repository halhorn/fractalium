//! セッション境界: フラクタル定義・編集中・レイアウトキャッシュ・ビュー要求を型で分ける。
//!
//! - [`fractal_state`]: 保存・共有の芯となる [`FractalState`]。
//! - [`edit_state`]: Undo／Placement と [`SnapGrid`]。
//! - [`layout_cache`]: UI 幾何の写し（ヒットテスト用矩形）。
//! - [`view_request`]: 一度で消化するビュー意図（カメラフィット等）。
//! - [`interaction_flags`]: 短寿命の入力ガード用フラグ。

pub mod edit_state;
pub mod fractal_state;
pub mod interaction_flags;
pub mod layout_cache;
pub mod view_request;

pub use edit_state::{PlacementDrag, PlacementState, SnapGrid, UndoStack};
pub use fractal_state::FractalState;
pub use interaction_flags::DoubleTapZoomActive;
pub use layout_cache::{CanvasLayout, ScreenRect, UiLayout};
pub use view_request::PendingResultCameraFit;
