//! Android JNI & NDK Native Bridge for Axomai Mobile Browser.
//! Exposes C-ABI / JNI functions to drive the Axomai Rust Engine from Android Java/Kotlin WebViews or Native Activities.

use std::os::raw::c_char;
use std::ffi::CStr;
use crate::engine::AxomaiEngine;

#[no_mangle]
pub extern "C" fn axomai_android_init() -> *mut AxomaiEngine {
    let engine = Box::new(AxomaiEngine::new());
    Box::into_raw(engine)
}

#[no_mangle]
pub extern "C" fn axomai_android_load_html(
    engine_ptr: *mut AxomaiEngine,
    html: *const c_char,
    width: f32,
    height: f32,
) -> bool {
    if engine_ptr.is_null() || html.is_null() {
        return false;
    }
    unsafe {
        let engine = &mut *engine_ptr;
        if let Ok(html_str) = CStr::from_ptr(html).to_str() {
            return engine.load_html(html_str, width, height).is_ok();
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn axomai_android_destroy(engine_ptr: *mut AxomaiEngine) {
    if !engine_ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(engine_ptr);
        }
    }
}
