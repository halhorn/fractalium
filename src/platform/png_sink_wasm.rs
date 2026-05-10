//! （Web 専用）[`crate::ports::png_export::PngExportSink`] の実装。モバイル寄り環境では Web Share、それ以外は `<a download>` で Blob を落とす。
//!
//! ## しないこと
//! - PNG を生成しない。受け取ったバイトをそのまま渡すのみ。

#![cfg(target_arch = "wasm32")]

use crate::ports::png_export::{AsyncMessageSlot, PngExportSink};

/// Web 向け [`PngExportSink`]。iOS 系列では優先して Web Share、その他はオブジェクト URL からのクリック保存。
#[derive(Clone, Copy, Default)]
pub(super) struct WasmPngSink;

/// 共有シート起動済みなら結果は後から `wasm_async_feedback` へ。その他は同期的に成功／失敗文を載せる。
impl PngExportSink for WasmPngSink {
    /// iOS 系ブラウザなら共有シート用の `try_share_png_sheet_with_catch` を試す。それ以外または未対応では Blob と `<a download>`。
    ///
    /// # パラメータ
    ///
    /// 共通の説明は [`PngExportSink::offer_png`](crate::ports::png_export::PngExportSink::offer_png)。
    ///
    /// * `share_sheet_text`: 共有時にのみ使用。トリム後・空ならファイル共有のみとなる。
    /// * `immediate_message`: 共有シートに乗れた場合、この return より前には通常触れない。
    ///
    /// # 戻り値
    ///
    /// なし。同期パスでは `immediate_message` に成功／エラー文言を代入。共有パスでは Promise 側が `wasm_async_feedback` に成功を書く。
    fn offer_png(
        &self,
        png_bytes: &[u8],
        suggested_filename: &str,
        share_sheet_text: Option<&str>,
        immediate_message: &mut Option<String>,
        wasm_async_feedback: AsyncMessageSlot,
    ) {
        let text = share_sheet_text.map(str::trim).filter(|s| !s.is_empty());
        if ua_looks_like_ios_family()
            && try_share_png_sheet_with_catch(
                png_bytes,
                suggested_filename,
                text,
                wasm_async_feedback,
            )
        {
            return;
        }
        match blob_download_anchor_click(png_bytes, suggested_filename) {
            Ok(()) => {
                *immediate_message = Some("Image saved".to_string());
            }
            Err(e) => {
                *immediate_message = Some(format!("Could not save image ({e})"));
            }
        }
    }
}

fn ua_looks_like_ios_family() -> bool {
    let Some(win) = web_sys::window() else {
        return false;
    };
    let ua = win
        .navigator()
        .user_agent()
        .unwrap_or_default()
        .to_ascii_lowercase();
    ua.contains("iphone")
        || ua.contains("ipad")
        || ua.contains("ipod")
        || (ua.contains("macintosh") && win.navigator().max_touch_points() > 0)
}

fn js_value_name(val: &wasm_bindgen::JsValue) -> Option<String> {
    js_sys::Reflect::get(val, &wasm_bindgen::JsValue::from_str("name"))
        .ok()?
        .as_string()
}

/// Web Share が使えれば起動する。Promise の成功／失敗は `share_done` へまたはフォールバックでダウンロードする。
fn try_share_png_sheet_with_catch(
    png_bytes: &[u8],
    filename: &str,
    share_text: Option<&str>,
    share_done: AsyncMessageSlot,
) -> bool {
    use js_sys::{Array, Function, Object, Reflect, Uint8Array};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};

    fn fill_share_data(files: &Array, share_text: Option<&str>, data: &Object) -> bool {
        if !Reflect::set(data, &JsValue::from_str("files"), files.as_ref()).unwrap_or(false) {
            return false;
        }
        if !Reflect::set(
            data,
            &JsValue::from_str("title"),
            &JsValue::from_str("Fractalium"),
        )
        .unwrap_or(false)
        {
            return false;
        }
        if let Some(t) = share_text {
            if !Reflect::set(data, &JsValue::from_str("text"), &JsValue::from_str(t))
                .unwrap_or(false)
            {
                return false;
            }
        }
        true
    }

    let Some(window) = web_sys::window() else {
        return false;
    };
    let nav = window.navigator();
    let Ok(share_prop) = Reflect::get(&nav, &JsValue::from_str("share")) else {
        return false;
    };
    if share_prop.is_undefined() || share_prop.is_null() {
        return false;
    }
    let Some(share_fn) = share_prop.dyn_ref::<Function>() else {
        return false;
    };

    let parts = Array::of1(Uint8Array::from(png_bytes).as_ref());
    let Ok(file) = web_sys::File::new_with_u8_array_sequence(&parts, filename) else {
        return false;
    };
    let files = Array::of1(file.as_ref());

    let Ok(can_prop) = Reflect::get(&nav, &JsValue::from_str("canShare")) else {
        return false;
    };
    if let Some(can_fn) = can_prop.dyn_ref::<Function>() {
        let data_probe = Object::new();
        if !Reflect::set(&data_probe, &JsValue::from_str("files"), files.as_ref()).unwrap_or(false)
        {
            return false;
        }
        if !Reflect::apply(
            can_fn,
            &JsValue::from(window.navigator()),
            &Array::of1(&data_probe),
        )
        .ok()
        .is_some_and(|v| v.is_truthy())
        {
            return false;
        }
    }

    let data = Object::new();
    if !fill_share_data(&files, share_text, &data) {
        return false;
    }

    let ret = match Reflect::apply(
        share_fn,
        &JsValue::from(window.navigator()),
        &Array::of1(&data),
    ) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let Some(promise) = ret.dyn_ref::<js_sys::Promise>() else {
        return false;
    };

    let bridge_ok = share_done.clone();
    let on_ok = Closure::wrap(Box::new(move |_v: JsValue| {
        if let Ok(mut g) = bridge_ok.lock() {
            *g = Some("Image saved".to_string());
        }
    }) as Box<dyn FnMut(JsValue)>);

    let bridge_fallback = share_done.clone();
    let png = png_bytes.to_vec();
    let filename_owned = filename.to_string();
    let on_err = Closure::wrap(Box::new(move |err: JsValue| {
        if js_value_name(&err).as_deref() == Some("AbortError") {
            return;
        }
        if blob_download_anchor_click(&png, &filename_owned).is_ok()
            && let Ok(mut g) = bridge_fallback.lock()
        {
            *g = Some("Image saved".to_string());
        }
    }) as Box<dyn FnMut(JsValue)>);

    let after_ok = promise.then(&on_ok);
    let _ = after_ok.catch(&on_err);
    on_ok.forget();
    on_err.forget();
    true
}

fn blob_download_anchor_click(png_bytes: &[u8], filename: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let arr = js_sys::Uint8Array::from(png_bytes);
    let parts = js_sys::Array::of1(arr.as_ref());
    let blob =
        web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(|e| format!("blob: {:?}", e))?;
    let url =
        web_sys::Url::create_object_url_with_blob(&blob).map_err(|e| format!("url: {:?}", e))?;
    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let document = window.document().ok_or_else(|| "no document".to_string())?;
    let link = document
        .create_element("a")
        .map_err(|e| format!("a: {:?}", e))?;
    link.set_attribute("href", &url)
        .map_err(|e| format!("href: {:?}", e))?;
    link.set_attribute("download", filename)
        .map_err(|e| format!("download: {:?}", e))?;
    let body = document
        .body()
        .ok_or_else(|| "no document body".to_string())?;
    body.append_child(&link)
        .map_err(|e| format!("append: {:?}", e))?;
    let html = link
        .dyn_ref::<web_sys::HtmlElement>()
        .ok_or_else(|| "a: not HtmlElement".to_string())?;
    html.click();
    let _ = body.remove_child(&link);
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}
