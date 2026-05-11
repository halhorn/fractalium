//! Result フラクタルの PNG 書き出し（オフスクリーン画像 + screenshot 読み戻し）。

use std::io::Cursor;
use std::sync::Arc;

use bevy::{
    camera::RenderTarget,
    camera::ScalingMode,
    ecs::observer::On,
    prelude::*,
    render::{
        render_resource::TextureFormat,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
};

use crate::app::session::FractalState;
use crate::bootstrap::result_export_layer;
use crate::ports::png_export::PngExportSink;
use crate::ui::canvas::result::navigation::result_export_projection;
use crate::ui::canvas::result::scene::{FractalExportMesh, rebuild_fractal_export_mesh};
use crate::ui::feedback::toast::DeferredToast;

/// メニュー経由で届く PNG をプラットフォームへ渡す。具象 trait は [`crate::ports::png_export::PngExportSink`]（実装は `platform`）。
#[derive(Resource, Clone)]
pub struct ResultImageOutlet(pub Arc<dyn PngExportSink + Send + Sync>);

/// Share メニューが開いたときに送信し、Result PNG の生成だけを開始する（保存は行わない）。
#[derive(Message)]
pub struct RequestResultImageExport;

/// 生成済み PNG。Web では `navigator.share` / ダウンロードがユーザー操作に直結する必要があるため、ここに溜めてから Download 押下で渡す。
#[derive(Resource)]
pub struct PreparedResultImage {
    pub state: PreparedResultImageState,
    /// Share サブメニューが 1 フレーム前まで開いていたか（開閉エッジ検出用）。
    pub share_menu_was_open: bool,
    /// `ResultExportBusy` と同期（egui システムのパラメータ数制限のためここから読む）。
    pub export_phase: ExportPhase,
}

impl Default for PreparedResultImage {
    fn default() -> Self {
        Self {
            state: PreparedResultImageState::None,
            share_menu_was_open: false,
            export_phase: ExportPhase::Idle,
        }
    }
}

pub enum PreparedResultImageState {
    None,
    Preparing,
    Ready { png: Vec<u8>, filename: String },
}

impl Default for PreparedResultImageState {
    fn default() -> Self {
        Self::None
    }
}

/// Download 押下時に同フレームで呼ぶ（user activation を維持するため）。
/// `share_sheet_text` は WASM / Web Share 向け（X 等の本文）。ネイティブ保存では未使用。
pub fn deliver_prepared_result_png(
    outlet: &ResultImageOutlet,
    prepared: &mut PreparedResultImage,
    deferred: &mut DeferredToast,
    share_sheet_text: Option<String>,
) {
    if let PreparedResultImageState::Ready { png, filename } = std::mem::take(&mut prepared.state) {
        let trimmed = share_sheet_text
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let pending = deferred.async_feedback_slot();
        outlet
            .0
            .offer_png(&png, &filename, trimmed, &mut deferred.message, pending);
    }
}

#[derive(Component)]
struct ResultExportCamera;

#[derive(Resource)]
struct ExportTargetImage(pub Handle<Image>);

/// 書き出し解像度。WebGL2 / 環境により 4096² + Unorm+sRGB ビューの組み合わせが wgpu パニックになるため WASM では控えめにする。
#[cfg(target_arch = "wasm32")]
const EXPORT_SIZE: u32 = 2048;
#[cfg(not(target_arch = "wasm32"))]
const EXPORT_SIZE: u32 = 4096;
const EXPORT_LINE_WIDTH_PX: f32 = 2.0;
/// オフスクリーン描画安定化まで待つフレーム数。
const EXPORT_WARMUP_FRAMES: u8 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportPhase {
    Idle,
    /// 残フレーム数（1 で次の進行時にスクショ）。
    Warm(u8),
    Capturing,
}

impl Default for ExportPhase {
    fn default() -> Self {
        ExportPhase::Idle
    }
}

#[derive(Default, Resource)]
pub struct ResultExportBusy(pub ExportPhase);

pub struct ResultExportPlugin;

impl Plugin for ResultExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RequestResultImageExport>()
            .init_resource::<ResultExportBusy>()
            .init_resource::<PreparedResultImage>()
            .add_systems(Startup, setup_result_export_camera)
            .add_systems(Update, result_export_pipeline);
    }
}

/// WebGL で `Unorm + 別フォーマットの texture view` により wgpu がパニックする報告があるため、WASM では単一 SRGB で作る。
fn new_export_render_target_image() -> Image {
    #[cfg(target_arch = "wasm32")]
    {
        Image::new_target_texture(
            EXPORT_SIZE,
            EXPORT_SIZE,
            TextureFormat::Rgba8UnormSrgb,
            None,
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Image::new_target_texture(
            EXPORT_SIZE,
            EXPORT_SIZE,
            TextureFormat::Rgba8Unorm,
            Some(TextureFormat::Rgba8UnormSrgb),
        )
    }
}

fn setup_result_export_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image_handle = images.add(new_export_render_target_image());

    commands.insert_resource(ExportTargetImage(image_handle.clone()));

    commands.spawn((
        ResultExportCamera,
        Camera2d,
        Camera {
            order: -2,
            clear_color: Color::srgb(0.08, 0.08, 0.10).into(),
            is_active: false,
            ..default()
        },
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 2.0,
                min_height: 2.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::default(),
        RenderTarget::Image(image_handle.clone().into()),
        result_export_layer(),
    ));
}

fn png_bytes_from_image(img: Image) -> Result<Vec<u8>, String> {
    let dyn_img = img
        .try_into_dynamic()
        .map_err(|e| format!("screenshot decode: {e}"))?;
    let rgb = dyn_img.to_rgb8();
    let mut buf = Cursor::new(Vec::new());
    rgb.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(buf.into_inner())
}

fn default_export_filename() -> String {
    let ms = export_filename_millis();
    format!("fractalium-result-{ms}.png")
}

#[cfg(not(target_arch = "wasm32"))]
fn export_filename_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// WebAssembly では `SystemTime::now` が利用できない（パニックする）。
#[cfg(target_arch = "wasm32")]
fn export_filename_millis() -> u128 {
    js_sys::Date::now() as u128
}

fn finalize_png_export_capture(
    capture: On<ScreenshotCaptured>,
    mut meshes: ResMut<Assets<Mesh>>,
    export_mesh_q: Query<&Mesh2d, With<FractalExportMesh>>,
    mut cameras: Query<&mut Camera, With<ResultExportCamera>>,
    mut export_busy: ResMut<ResultExportBusy>,
    mut deferred: ResMut<DeferredToast>,
    mut prepared: ResMut<PreparedResultImage>,
) {
    let img = capture.image.clone();
    match png_bytes_from_image(img) {
        Ok(bytes) => {
            let filename = default_export_filename();
            prepared.state = PreparedResultImageState::Ready {
                png: bytes,
                filename,
            };
        }
        Err(e) => {
            prepared.state = PreparedResultImageState::None;
            deferred.message = Some(e);
        }
    }

    export_busy.0 = ExportPhase::Idle;

    prepared.export_phase = ExportPhase::Idle;

    if let Ok(mut cam) = cameras.single_mut() {
        cam.is_active = false;
    }
    if let Ok(mesh2d) = export_mesh_q.single() {
        if let Some(m) = meshes.get_mut(&mesh2d.0) {
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
        }
    }
}

fn result_export_pipeline(
    mut commands: Commands,
    mut msgs: MessageReader<RequestResultImageExport>,
    mut export_busy: ResMut<ResultExportBusy>,
    export_target: Res<ExportTargetImage>,
    mut assets: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    fractal_export_q: Query<&Mesh2d, With<FractalExportMesh>>,
    mut cameras: Query<(&mut Camera, &mut Projection, &mut Transform), With<ResultExportCamera>>,
    state: Res<FractalState>,
    mut deferred: ResMut<DeferredToast>,
    mut prepared: ResMut<PreparedResultImage>,
) {
    match export_busy.0 {
        ExportPhase::Idle => {
            for _ in msgs.read() {
                prepared.state = PreparedResultImageState::Preparing;
                start_export_prep(
                    &mut assets,
                    &mut meshes,
                    &fractal_export_q,
                    &mut cameras,
                    &state,
                    &export_target,
                    &mut export_busy,
                    &mut deferred,
                    &mut prepared,
                );
            }
        }
        ExportPhase::Warm(frames_left) => {
            let _ = msgs.read();
            if frames_left <= 1 {
                let h = export_target.0.clone();
                commands
                    .spawn(Screenshot::image(h))
                    .observe(finalize_png_export_capture);
                export_busy.0 = ExportPhase::Capturing;
            } else {
                export_busy.0 = ExportPhase::Warm(frames_left - 1);
            }
        }
        ExportPhase::Capturing => {
            let _ = msgs.read();
        }
    }

    prepared.export_phase = export_busy.0;
}

fn start_export_prep(
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    fractal_export_q: &Query<&Mesh2d, With<FractalExportMesh>>,
    cameras: &mut Query<(&mut Camera, &mut Projection, &mut Transform), With<ResultExportCamera>>,
    state: &FractalState,
    export_target: &ExportTargetImage,
    export_busy: &mut ResultExportBusy,
    deferred: &mut DeferredToast,
    prepared: &mut PreparedResultImage,
) {
    if !matches!(export_busy.0, ExportPhase::Idle) {
        deferred.message = Some("Image export already in progress".into());
        prepared.state = PreparedResultImageState::None;
        return;
    }

    let Ok((mut cam, mut proj, mut tf)) = cameras.single_mut() else {
        prepared.state = PreparedResultImageState::None;
        return;
    };

    let handle = export_target.0.clone();
    let Some(tex) = images.get_mut(&handle) else {
        deferred.message = Some("Export render target missing".into());
        prepared.state = PreparedResultImageState::None;
        return;
    };

    *tex = new_export_render_target_image();

    let (new_proj, new_tf) = result_export_projection(state, EXPORT_SIZE);
    let scale_fit = if let Projection::Orthographic(o) = &new_proj {
        o.scale
    } else {
        1.0
    };
    *proj = new_proj;
    *tf = new_tf;

    let half_line_world = EXPORT_LINE_WIDTH_PX * scale_fit / EXPORT_SIZE as f32;

    let Ok(mesh2d) = fractal_export_q.single() else {
        deferred.message = Some("Export mesh missing".into());
        prepared.state = PreparedResultImageState::None;
        return;
    };
    let Some(mesh) = meshes.get_mut(&mesh2d.0) else {
        deferred.message = Some("Export mesh asset missing".into());
        prepared.state = PreparedResultImageState::None;
        return;
    };
    rebuild_fractal_export_mesh(mesh, state, half_line_world);

    cam.is_active = true;
    export_busy.0 = ExportPhase::Warm(EXPORT_WARMUP_FRAMES);
}
