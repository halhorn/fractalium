//! `FractalPreset` ごとの同梱 PNG を `include_bytes!` から解決し、egui テクスチャへ遅延変換する。
//!
//! ビジネスロジック（どのフラクタルにするか等）は載せず、キャッシュのみを担う。

use std::collections::HashMap;
use std::io::Cursor;

use bevy::prelude::Resource;
use bevy_egui::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::ImageReader;

use crate::fractal_presets::FractalPreset;

/// サムネイル用 PNG を 1 度だけデコードして [`TextureHandle`] に載せたキャッシュ。
#[derive(Resource)]
pub struct PresetThumbnailCache {
    /// 既定「新規」タイル画像。
    new_tile: Option<TextureHandle>,
    /// `FractalPreset` の識別子ごとの画像。
    by_preset: HashMap<FractalPreset, Option<TextureHandle>>,
}

impl Default for PresetThumbnailCache {
    fn default() -> Self {
        let mut by_preset = HashMap::new();
        for &p in FractalPreset::ALL {
            by_preset.insert(p, None);
        }
        Self {
            new_tile: None,
            by_preset,
        }
    }
}

impl PresetThumbnailCache {
    /// 「新規」タイルがあればそれを、`load_texture` が失敗したら `None`。
    ///
    /// # 引数
    /// - `ctx` — 現在フレームの egui [`egui::Context`]。
    ///
    /// # 戻り値
    /// 読み込み済みハンドルの参照または `None`。
    pub fn texture_new_tile<'a>(&'a mut self, ctx: &egui::Context) -> Option<&'a TextureHandle> {
        if self.new_tile.is_none() {
            self.new_tile = bytes_to_texture(ctx, "preset_thumb_new", NEW_TILE_BYTES);
        }
        self.new_tile.as_ref()
    }

    /// 既定プリセットごとの PNG があれば返す。
    ///
    /// # 引数
    /// - `ctx` — egui のコンテキスト。
    /// - `preset` — 列挙子。
    ///
    /// # 戻り値
    /// 読み込み済みまたは失敗確定済みとして `Some`/`None`。
    pub fn texture_for_preset<'a>(
        &'a mut self,
        ctx: &egui::Context,
        preset: FractalPreset,
    ) -> Option<&'a TextureHandle> {
        let slot = self
            .by_preset
            .get_mut(&preset)
            .expect("preset thumbnails map seeded with ALL");
        if slot.is_none() {
            *slot = bytes_to_texture(ctx, &texture_key_for_preset(preset), png_bytes_for_preset(preset));
        }
        slot.as_ref()
    }
}

/// `OnEnter(AppScreen::PresetPicker)` で「初回フォーカス要求」をやり直すためのフラグ。
#[derive(Resource, Debug)]
pub struct PresetPickerNeedsInitialFocus(pub bool);

impl Default for PresetPickerNeedsInitialFocus {
    fn default() -> Self {
        Self(true)
    }
}

fn texture_key_for_preset(p: FractalPreset) -> String {
    format!("preset_thumb_{p:?}")
}

fn png_bytes_for_preset(preset: FractalPreset) -> &'static [u8] {
    match preset {
        FractalPreset::SierpinskiTriangle => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/sierpinski_triangle.png"
        )),
        FractalPreset::KochCurve => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/koch_curve.png"
        )),
        FractalPreset::Vicsek => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/vicsek.png"
        )),
        FractalPreset::HeighwayDragon => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/heighway_dragon.png"
        )),
        FractalPreset::LevyCCurve => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/levy_c.png"
        )),
        FractalPreset::SierpinskiCarpet => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/sierpinski_carpet.png"
        )),
        FractalPreset::PythagorasTree => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/pythagoras_tree.png"
        )),
        FractalPreset::SierpinskiHexagon => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/sierpinski_hexagon.png"
        )),
        FractalPreset::SierpinskiStar => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/sierpinski_star.png"
        )),
        FractalPreset::BinaryFractalTree => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/binary_fractal_tree.png"
        )),
        FractalPreset::Terdragon => include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fractalium/preset_thumbnails/terdragon.png"
        )),
    }
}

const NEW_TILE_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fractalium/preset_thumbnails/new.png"
));

fn bytes_to_texture(ctx: &egui::Context, key: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let dyn_img = reader.decode().ok()?;
    let rgba = dyn_img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(key, color, TextureOptions::LINEAR))
}
