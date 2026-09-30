#ifndef AXOMAI_ENGINE_BRIDGE_H
#define AXOMAI_ENGINE_BRIDGE_H

#include <cstdint>
#include <cstddef>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint32_t cmd_type; // 0 = DrawRect, 1 = DrawText, 2 = DrawImage, 3 = DrawInput, 4 = DrawButton
    float x;
    float y;
    float width;
    float height;
    char color[64];
    char text[512];
    char href[512];
    float font_size;
    bool font_weight_bold;
    bool font_style_italic;
    bool is_focused;
    const uint8_t* image_ptr;
    size_t image_len;
} FFIDisplayCommand;

void* axomai_engine_create();
void axomai_engine_free(void* engine_ptr);
bool axomai_engine_load_url(void* engine_ptr, const char* url, float viewport_w, float viewport_h);
bool axomai_engine_load_html(void* engine_ptr, const char* html, float viewport_w, float viewport_h);
size_t axomai_engine_get_display_count(void* engine_ptr);
bool axomai_engine_get_display_command(void* engine_ptr, size_t index, FFIDisplayCommand* out_cmd);
float axomai_engine_get_max_scroll(void* engine_ptr);
bool axomai_engine_handle_click(void* engine_ptr, float click_x, float click_y, float scroll_y, char* out_url_buf, size_t max_len);
bool axomai_engine_handle_key(void* engine_ptr, const char* key_c_str, char* out_url_buf, size_t max_len);
bool axomai_engine_process_event_loop(void* engine_ptr, float viewport_w, float viewport_h);

#ifdef __cplusplus
}
#endif

#endif // AXOMAI_ENGINE_BRIDGE_H
