//! Result フラクタルの PNG 書き出し（オフスクリーン画像 + screenshot 読み戻し）。

use std::io::Cursor;

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

use crate::{
    state::FractalState,
    toast::DeferredToast,
    fractal::{FractalExportMesh, rebuild_fractal_export_mesh},
    view::result_export_projection,
};

/// 「Download image」から送信する。
#[derive(Message)]
pub struct RequestResultImageExport;

#[derive(Component)]
struct ResultExportCamera;

#[derive(Resource)]
struct ExportTargetImage(pub Handle<Image>);

/// 書き出し解像度。WebGL2 / 環境により 4096² + Unorm+sRGB ビューの組み合わせが wgpu パニックになるため WASM では控えめにする。
#[cfg(target_arch = "wasm32")]
const EXPORT_SIZE: u32 = 2048;
#[cfg(not(target_arch = "wasm32"))]
const EXPORT_SIZE: u32 = 4096;
const EXPORT_LINE_WIDTH_PX: f32 = 4.0;
/// オフスクリーン描画安定化まで待つフレーム数。
const EXPORT_WARMUP_FRAMES: u8 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportPhase {
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
struct ResultExportBusy(pub ExportPhase);

pub struct ResultExportPlugin;

impl Plugin for ResultExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RequestResultImageExport>()
            .init_resource::<ResultExportBusy>()
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
        crate::result_export_layer(),
    ));
}

fn png_bytes_from_image(img: Image) -> Result<Vec<u8>, String> {
    let dyn_img = img.try_into_dynamic().map_err(|e| format!("screenshot decode: {e}"))?;
    let rgb = dyn_img.to_rgb8();
    let mut buf = Cursor::new(Vec::new());
    rgb.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(buf.into_inner())
}

fn default_export_filename() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("fractalium-result-{ms}.png")
}

#[cfg(not(target_arch = "wasm32"))]
fn offer_png(png_bytes: &[u8], deferred: &mut DeferredToast) {
    match rfd::FileDialog::new()
        .set_file_name(&default_export_filename())
        .save_file()
    {
        Some(path) => match std::fs::write(&path, png_bytes) {
            Ok(()) => {
                deferred.0 = Some("Image saved".to_string());
            }
            Err(e) => {
                deferred.0 = Some(format!("Save failed: {e}"));
            }
        },
        None => {}
    }
}

#[cfg(target_arch = "wasm32")]
fn try_web_share_png(png_bytes: &[u8]) -> bool {
    use js_sys::{Array, Function, Reflect, Uint8Array};
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return false;
    };
    let nav = window.navigator();
    let Ok(share) = Reflect::get(&nav, &JsValue::from_str("share")) else {
        return false;
    };
    if share.is_undefined() || share.is_null() {
        return false;
    }
    let Some(share_fn) = share.dyn_ref::<Function>() else {
        return false;
    };

    let parts = Array::of1(Uint8Array::from(png_bytes).as_ref());
    let Ok(file) = web_sys::File::new_with_u8_array_sequence(&parts, "fractalium-result.png") else {
        return false;
    };
    let files = Array::of1(file.as_ref());
    let data = js_sys::Object::new();
    let Ok(true) = Reflect::set(&data, &JsValue::from_str("files"), files.as_ref()) else {
        return false;
    };
    let Ok(true) = Reflect::set(
        &data,
        &JsValue::from_str("title"),
        &JsValue::from_str("Fractalium"),
    ) else {
        return false;
    };
    let arg = JsValue::from(data);
    let this_arg = JsValue::from(nav);
    Reflect::apply(share_fn, &this_arg, &Array::of1(&arg))
        .map(|_| true)
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn offer_png(png_bytes: &[u8], deferred: &mut DeferredToast) {
    if try_web_share_png(png_bytes) {
        deferred.0 = Some("Share sheet opened".to_string());
        return;
    }
    match wasm_blob_download_png(png_bytes, &default_export_filename()) {
        Ok(()) => {
            deferred.0 = Some("Image download started".to_string());
        }
        Err(e) => {
            deferred.0 = Some(format!("Could not save image ({e})"));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn wasm_blob_download_png(png_bytes: &[u8], filename: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let arr = js_sys::Uint8Array::from(png_bytes);
    let parts = js_sys::Array::of1(arr.as_ref());
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(|e| format!("blob: {:?}", e))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|e| format!("url: {:?}", e))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;
    let link = document.create_element("a").map_err(|e| format!("a: {:?}", e))?;
    link.set_attribute("href", &url).map_err(|e| format!("href: {:?}", e))?;
    link.set_attribute("download", filename).map_err(|e| format!("download: {:?}", e))?;
    let html = link.dyn_into::<web_sys::HtmlElement>().map_err(|e| format!("html: {:?}", e))?;
    html.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

fn finalize_png_export_capture(
    capture: On<ScreenshotCaptured>,
    mut meshes: ResMut<Assets<Mesh>>,
    export_mesh_q: Query<&Mesh2d, With<FractalExportMesh>>,
    mut cameras: Query<&mut Camera, With<ResultExportCamera>>,
    mut export_busy: ResMut<ResultExportBusy>,
    mut deferred: ResMut<DeferredToast>,
) {
    let img = capture.image.clone();
    match png_bytes_from_image(img) {
        Ok(bytes) => {
            offer_png(&bytes, &mut *deferred);
        }
        Err(e) => {
            deferred.0 = Some(e);
        }
    }

    export_busy.0 = ExportPhase::Idle;

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
) {
    match export_busy.0 {
        ExportPhase::Idle => {
            for _ in msgs.read() {
                start_export_prep(
                    &mut assets,
                    &mut meshes,
                    &fractal_export_q,
                    &mut cameras,
                    &state,
                    &export_target,
                    &mut export_busy,
                    &mut deferred,
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
) {
    if !matches!(export_busy.0, ExportPhase::Idle) {
        deferred.0 = Some("Image export already in progress".into());
        return;
    }

    let Ok((mut cam, mut proj, mut tf)) = cameras.single_mut() else {
        return;
    };

    let handle = export_target.0.clone();
    let Some(tex) = images.get_mut(&handle) else {
        deferred.0 = Some("Export render target missing".into());
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
        deferred.0 = Some("Export mesh missing".into());
        return;
    };
    let Some(mesh) = meshes.get_mut(&mesh2d.0) else {
        deferred.0 = Some("Export mesh asset missing".into());
        return;
    };
    rebuild_fractal_export_mesh(mesh, state, half_line_world);

    cam.is_active = true;
    export_busy.0 = ExportPhase::Warm(EXPORT_WARMUP_FRAMES);
}
