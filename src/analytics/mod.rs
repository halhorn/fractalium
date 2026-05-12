//! Web（WASM）向けに `index.html` の gtag.js へカスタムイベントを送る。
//!
//! `gtag` が未定義の場合は送らずに終える。ネイティブビルドでは呼び出しはすべて no-op。
//! イベント名・パラメータキーは呼び出し側で文字列または定数として持つ。

/// GA4 にカスタムイベントを送る。ネイティブでは何もしない。
///
/// # 引数
/// - `name` — GA4 のイベント名（例: `"fractalium_undo"`）。呼び出し側で指定する。
/// - `params` — イベントパラメータのキーと値（ASCII を想定）。
pub fn track_event(name: &str, params: &[(&str, &str)]) {
    #[cfg(target_arch = "wasm32")]
    track_event_wasm(name, params);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (name, params);
    }
}

#[cfg(target_arch = "wasm32")]
fn track_event_wasm(name: &str, params: &[(&str, &str)]) {
    use js_sys::{Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(gtag_val) = Reflect::get(&window, &JsValue::from_str("gtag")) else {
        return;
    };
    if !gtag_val.is_function() {
        return;
    };
    let Some(gtag_fn) = gtag_val.dyn_ref::<js_sys::Function>() else {
        return;
    };

    let obj = Object::new();
    for &(k, v) in params {
        let _ = Reflect::set(&obj, &JsValue::from_str(k), &JsValue::from_str(v));
    }

    let this = JsValue::NULL;
    let _ = gtag_fn.call3(
        &this,
        &JsValue::from_str("event"),
        &JsValue::from_str(name),
        &JsValue::from(&obj),
    );
}
