use crate::engine::AxomaiEngine;
use crate::painter::DisplayCommand;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::slice;

#[repr(C)]
pub struct FFIDisplayCommand {
    pub cmd_type: u32, // 0 = DrawRect, 1 = DrawText, 2 = DrawImage, 3 = DrawInput, 4 = DrawButton
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [c_char; 64],
    pub text: [c_char; 512],
    pub href: [c_char; 512],
    pub font_size: f32,
    pub font_weight_bold: bool,
    pub font_style_italic: bool,
    pub is_focused: bool,
    pub image_ptr: *const u8,
    pub image_len: usize,
}

impl Default for FFIDisplayCommand {
    fn default() -> Self {
        FFIDisplayCommand {
            cmd_type: 0,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            color: [0; 64],
            text: [0; 512],
            href: [0; 512],
            font_size: 16.0,
            font_weight_bold: false,
            font_style_italic: false,
            is_focused: false,
            image_ptr: std::ptr::null(),
            image_len: 0,
        }
    }
}

fn copy_to_c_buf(src: &str, dest: &mut [c_char]) {
    let bytes = src.as_bytes();
    let max_len = dest.len() - 1;
    let len = bytes.len().min(max_len);
    for i in 0..len {
        dest[i] = bytes[i] as c_char;
    }
    dest[len] = 0;
}

#[no_mangle]
pub extern "C" fn axomai_engine_create() -> *mut c_void {
    let engine = Box::new(AxomaiEngine::new());
    Box::into_raw(engine) as *mut c_void
}

#[no_mangle]
pub extern "C" fn axomai_engine_free(engine_ptr: *mut c_void) {
    if !engine_ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(engine_ptr as *mut AxomaiEngine);
        }
    }
}

#[no_mangle]
pub extern "C" fn axomai_engine_load_url(
    engine_ptr: *mut c_void,
    url_c_str: *const c_char,
    viewport_w: f32,
    viewport_h: f32,
) -> bool {
    if engine_ptr.is_null() || url_c_str.is_null() {
        return false;
    }
    let engine = unsafe { &mut *(engine_ptr as *mut AxomaiEngine) };
    if let Ok(c_str) = unsafe { CStr::from_ptr(url_c_str) }.to_str() {
        return engine.load_url(c_str, viewport_w, viewport_h).is_ok();
    }
    false
}

#[no_mangle]
pub extern "C" fn axomai_engine_load_html(
    engine_ptr: *mut c_void,
    html_c_str: *const c_char,
    viewport_w: f32,
    viewport_h: f32,
) -> bool {
    if engine_ptr.is_null() || html_c_str.is_null() {
        return false;
    }
    let engine = unsafe { &mut *(engine_ptr as *mut AxomaiEngine) };
    if let Ok(c_str) = unsafe { CStr::from_ptr(html_c_str) }.to_str() {
        return engine.load_html(c_str, viewport_w, viewport_h).is_ok();
    }
    false
}

#[no_mangle]
pub extern "C" fn axomai_engine_get_display_count(engine_ptr: *mut c_void) -> usize {
    if engine_ptr.is_null() {
        return 0;
    }
    let engine = unsafe { &*(engine_ptr as *mut AxomaiEngine) };
    engine.display_list.len()
}

#[no_mangle]
pub extern "C" fn axomai_engine_get_display_command(
    engine_ptr: *mut c_void,
    index: usize,
    out_cmd: *mut FFIDisplayCommand,
) -> bool {
    if engine_ptr.is_null() || out_cmd.is_null() {
        return false;
    }
    let engine = unsafe { &*(engine_ptr as *mut AxomaiEngine) };
    if index >= engine.display_list.len() {
        return false;
    }

    let ffi_cmd = unsafe { &mut *out_cmd };
    *ffi_cmd = FFIDisplayCommand::default();

    match &engine.display_list[index] {
        DisplayCommand::DrawRect {
            x1,
            y1,
            x2,
            y2,
            color,
        } => {
            ffi_cmd.cmd_type = 0;
            ffi_cmd.x = *x1;
            ffi_cmd.y = *y1;
            ffi_cmd.width = *x2 - *x1;
            ffi_cmd.height = *y2 - *y1;
            copy_to_c_buf(color, &mut ffi_cmd.color);
        }
        DisplayCommand::DrawText {
            x,
            y,
            width,
            height,
            text,
            font_size,
            font_weight,
            font_style,
            color,
            href,
        } => {
            ffi_cmd.cmd_type = 1;
            ffi_cmd.x = *x;
            ffi_cmd.y = *y;
            ffi_cmd.width = *width;
            ffi_cmd.height = *height;
            copy_to_c_buf(text, &mut ffi_cmd.text);
            copy_to_c_buf(color, &mut ffi_cmd.color);
            copy_to_c_buf(href, &mut ffi_cmd.href);
            ffi_cmd.font_size = *font_size;
            ffi_cmd.font_weight_bold = font_weight == "bold";
            ffi_cmd.font_style_italic = font_style == "italic";
        }
        DisplayCommand::DrawImage {
            x,
            y,
            width,
            height,
            image_bytes,
        } => {
            ffi_cmd.cmd_type = 2;
            ffi_cmd.x = *x;
            ffi_cmd.y = *y;
            ffi_cmd.width = *width;
            ffi_cmd.height = *height;
            ffi_cmd.image_ptr = image_bytes.as_ptr();
            ffi_cmd.image_len = image_bytes.len();
        }
        DisplayCommand::DrawInput {
            x,
            y,
            width,
            height,
            value,
            placeholder,
            is_focused,
        } => {
            ffi_cmd.cmd_type = 3;
            ffi_cmd.x = *x;
            ffi_cmd.y = *y;
            ffi_cmd.width = *width;
            ffi_cmd.height = *height;
            let display_txt = if value.is_empty() { placeholder } else { value };
            copy_to_c_buf(display_txt, &mut ffi_cmd.text);
            ffi_cmd.is_focused = *is_focused;
        }
        DisplayCommand::DrawButton {
            x,
            y,
            width,
            height,
            label,
        } => {
            ffi_cmd.cmd_type = 4;
            ffi_cmd.x = *x;
            ffi_cmd.y = *y;
            ffi_cmd.width = *width;
            ffi_cmd.height = *height;
            copy_to_c_buf(label, &mut ffi_cmd.text);
        }
    }
    true
}

#[no_mangle]
pub extern "C" fn axomai_engine_get_max_scroll(engine_ptr: *mut c_void) -> f32 {
    if engine_ptr.is_null() {
        return 0.0;
    }
    let engine = unsafe { &*(engine_ptr as *mut AxomaiEngine) };
    engine.max_scroll_y
}

#[no_mangle]
pub extern "C" fn axomai_engine_handle_click(
    engine_ptr: *mut c_void,
    click_x: f32,
    click_y: f32,
    scroll_y: f32,
    out_url_buf: *mut c_char,
    max_len: usize,
) -> bool {
    if engine_ptr.is_null() {
        return false;
    }
    let engine = unsafe { &mut *(engine_ptr as *mut AxomaiEngine) };
    if let Some(target_url) = engine.handle_click(click_x, click_y, scroll_y) {
        if !out_url_buf.is_null() && max_len > 0 {
            let slice = unsafe { slice::from_raw_parts_mut(out_url_buf, max_len) };
            copy_to_c_buf(&target_url, slice);
        }
        return true;
    }
    false
}

#[no_mangle]
pub extern "C" fn axomai_engine_handle_key(
    engine_ptr: *mut c_void,
    key_c_str: *const c_char,
    out_url_buf: *mut c_char,
    max_len: usize,
) -> bool {
    if engine_ptr.is_null() || key_c_str.is_null() {
        return false;
    }
    let engine = unsafe { &mut *(engine_ptr as *mut AxomaiEngine) };
    if let Ok(key_str) = unsafe { CStr::from_ptr(key_c_str) }.to_str() {
        if let Some(target_url) = engine.handle_key(key_str) {
            if !out_url_buf.is_null() && max_len > 0 {
                let slice = unsafe { slice::from_raw_parts_mut(out_url_buf, max_len) };
                copy_to_c_buf(&target_url, slice);
            }
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn axomai_engine_process_event_loop(
    engine_ptr: *mut c_void,
    viewport_w: f32,
    viewport_h: f32,
) -> bool {
    if engine_ptr.is_null() {
        return false;
    }
    let engine = unsafe { &mut *(engine_ptr as *mut AxomaiEngine) };
    engine.process_event_loop(viewport_w, viewport_h)
}

#[no_mangle]
pub extern "C" fn axomai_engine_has_pending_events(engine_ptr: *mut c_void) -> bool {
    if engine_ptr.is_null() {
        return false;
    }
    let engine = unsafe { &*(engine_ptr as *mut AxomaiEngine) };
    engine.has_pending_events()
}

