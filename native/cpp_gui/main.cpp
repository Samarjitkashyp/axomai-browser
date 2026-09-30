#define UNICODE
#define _UNICODE
#include <windows.h>
#include <gdiplus.h>
#include <iostream>
#include <vector>
#include <string>
#include "AxomaiEngineBridge.h"

#pragma comment(lib, "gdiplus.lib")

using namespace Gdiplus;

// Dynamic function pointers for Rust Engine DLL
typedef void* (*fn_axomai_engine_create)();
typedef void (*fn_axomai_engine_free)(void*);
typedef bool (*fn_axomai_engine_load_url)(void*, const char*, float, float);
typedef bool (*fn_axomai_engine_load_html)(void*, const char*, float, float);
typedef size_t (*fn_axomai_engine_get_display_count)(void*);
typedef bool (*fn_axomai_engine_get_display_command)(void*, size_t, FFIDisplayCommand*);
typedef float (*fn_axomai_engine_get_max_scroll)(void*);
typedef bool (*fn_axomai_engine_handle_click)(void*, float, float, float, char*, size_t);
typedef bool (*fn_axomai_engine_handle_key)(void*, const char*, char*, size_t);
typedef bool (*fn_axomai_engine_process_event_loop)(void*, float, float);
typedef bool (*fn_axomai_engine_has_pending_events)(void*);

static fn_axomai_engine_create p_axomai_engine_create = NULL;
static fn_axomai_engine_free p_axomai_engine_free = NULL;
static fn_axomai_engine_load_url p_axomai_engine_load_url = NULL;
static fn_axomai_engine_load_html p_axomai_engine_load_html = NULL;
static fn_axomai_engine_get_display_count p_axomai_engine_get_display_count = NULL;
static fn_axomai_engine_get_display_command p_axomai_engine_get_display_command = NULL;
static fn_axomai_engine_get_max_scroll p_axomai_engine_get_max_scroll = NULL;
static fn_axomai_engine_handle_click p_axomai_engine_handle_click = NULL;
static fn_axomai_engine_handle_key p_axomai_engine_handle_key = NULL;
static fn_axomai_engine_process_event_loop p_axomai_engine_process_event_loop = NULL;
static fn_axomai_engine_has_pending_events p_axomai_engine_has_pending_events = NULL;

static HMODULE g_hEngineDll = NULL;
static void* g_engine = NULL;

static HWND g_hAddrEntry = NULL;
static float g_scroll_y = 0.0f;
static float g_max_scroll_y = 0.0f;
static std::vector<std::string> g_history;
static int g_history_idx = -1;

const char* DEFAULT_PAGE_HTML = R"HTML(
<html>
  <head>
    <style>
      body { font-family: 'Segoe UI', sans-serif; background-color: #ffffff; margin: 0px; color: #202124; }
      .hero { text-align: center; margin-top: 40px; margin-bottom: 20px; }
      .title { font-size: 36px; font-weight: bold; color: #1a73e8; margin-bottom: 10px; }
      .subtitle { font-size: 14px; color: #5f6368; }
      .search-box { margin-top: 25px; margin-bottom: 30px; text-align: center; }
      .container { max-width: 800px; margin-left: auto; margin-right: auto; padding: 20px; }
      .card { background-color: #f8f9fa; border: 1px solid #dadce0; border-radius: 12px; padding: 20px; margin-bottom: 20px; }
      h2 { color: #1a73e8; font-size: 20px; margin-top: 0px; }
      p { color: #3c4043; font-size: 14px; margin-top: 8px; line-height: 1.5; }
      b { color: #1a73e8; }
      a { color: #1a73e8; text-decoration: none; font-weight: bold; }
    </style>
  </head>
  <body>
    <div class="hero">
      <div class="title">🚀 Axomai Browser (C++ & Rust Engine)</div>
      <div class="subtitle">High Performance Native Browser Pipeline</div>
    </div>
    <div class="container">
      <div class="search-box">
        <p><b>Search the Web or Enter URL:</b></p>
        <p>
          <input placeholder="Search Google or type a URL..." value="" width="480px" height="42px" />
          <button width="90px" height="42px">Search</button>
        </p>
      </div>
      <div class="card">
        <h2>⚡ Native Subsystems Status</h2>
        <p>1. 🦀 <b>Rust Core Engine:</b> High performance DOM, CSS Cascade & Layout.</p>
        <p>2. ⚡ <b>C++ Win32 & GDI+:</b> Native window GUI & Double-buffered Rendering.</p>
        <p>3. 🌐 <b>Networking:</b> SSL/TLS HTTP & local file/data URL parsing.</p>
        <p>4. 📝 <b>Form Controls:</b> Chrome Pill inputs & Canvas Typing.</p>
        <script>
          var statusMsg = "5. ⚡ JavaScript Engine: Inline script execution in Rust!";
          document.write("<p><b>" + statusMsg + "</b></p>");
        </script>
      </div>
      <div class="card">
        <h2>🌐 Quick Shortcuts</h2>
        <p>👉 <a href="https://example.org">Example.org</a> • <a href="https://wikipedia.org">Wikipedia</a> • <a href="https://github.com/Samarjitkashyp/axomai-browser">Axomai GitHub Repo</a></p>
      </div>
    </div>
  </body>
</html>
)HTML";

bool LoadEngineDll() {
    wchar_t exePath[MAX_PATH];
    GetModuleFileNameW(NULL, exePath, MAX_PATH);
    std::wstring exeDir(exePath);
    size_t lastSlash = exeDir.rfind(L'\\');
    if (lastSlash != std::wstring::npos) {
        exeDir = exeDir.substr(0, lastSlash + 1);
    }

    std::vector<std::wstring> dllPaths = {
        exeDir + L"axomai_engine.dll",
        exeDir + L"..\\rust_engine\\target\\release\\axomai_engine.dll",
        exeDir + L"..\\..\\native\\rust_engine\\target\\release\\axomai_engine.dll",
        L"axomai_engine.dll",
        L"native\\cpp_gui\\axomai_engine.dll",
        L"native\\rust_engine\\target\\release\\axomai_engine.dll"
    };

    for (const auto& path : dllPaths) {
        g_hEngineDll = LoadLibraryW(path.c_str());
        if (g_hEngineDll) break;
    }

    if (!g_hEngineDll) {
        DWORD err = GetLastError();
        wchar_t msg[512];
        swprintf_s(msg, 512, L"Could not load axomai_engine.dll!\nError Code: %lu\nExe Directory: %s", err, exeDir.c_str());
        MessageBoxW(NULL, msg, L"Axomai Engine Error", MB_ICONERROR);
        return false;
    }

    p_axomai_engine_create = (fn_axomai_engine_create)GetProcAddress(g_hEngineDll, "axomai_engine_create");
    p_axomai_engine_free = (fn_axomai_engine_free)GetProcAddress(g_hEngineDll, "axomai_engine_free");
    p_axomai_engine_load_url = (fn_axomai_engine_load_url)GetProcAddress(g_hEngineDll, "axomai_engine_load_url");
    p_axomai_engine_load_html = (fn_axomai_engine_load_html)GetProcAddress(g_hEngineDll, "axomai_engine_load_html");
    p_axomai_engine_get_display_count = (fn_axomai_engine_get_display_count)GetProcAddress(g_hEngineDll, "axomai_engine_get_display_count");
    p_axomai_engine_get_display_command = (fn_axomai_engine_get_display_command)GetProcAddress(g_hEngineDll, "axomai_engine_get_display_command");
    p_axomai_engine_get_max_scroll = (fn_axomai_engine_get_max_scroll)GetProcAddress(g_hEngineDll, "axomai_engine_get_max_scroll");
    p_axomai_engine_handle_click = (fn_axomai_engine_handle_click)GetProcAddress(g_hEngineDll, "axomai_engine_handle_click");
    p_axomai_engine_handle_key = (fn_axomai_engine_handle_key)GetProcAddress(g_hEngineDll, "axomai_engine_handle_key");
    p_axomai_engine_process_event_loop = (fn_axomai_engine_process_event_loop)GetProcAddress(g_hEngineDll, "axomai_engine_process_event_loop");
    p_axomai_engine_has_pending_events = (fn_axomai_engine_has_pending_events)GetProcAddress(g_hEngineDll, "axomai_engine_has_pending_events");

    return p_axomai_engine_create && p_axomai_engine_load_url;
}

Color ParseHexColor(const char* hex) {
    if (!hex || hex[0] != '#') return Color(255, 0, 0, 0);
    std::string s(hex + 1);
    if (s.length() == 6) {
        int r = std::stoi(s.substr(0, 2), nullptr, 16);
        int g = std::stoi(s.substr(2, 2), nullptr, 16);
        int b = std::stoi(s.substr(4, 2), nullptr, 16);
        return Color(255, r, g, b);
    }
    if (s == "red") return Color(255, 255, 0, 0);
    if (s == "blue") return Color(255, 0, 0, 255);
    if (s == "white") return Color(255, 255, 255, 255);
    if (s == "black") return Color(255, 0, 0, 0);
    return Color(255, 32, 33, 36);
}

void LoadUrlInEngine(const std::string& url_str, HWND hWnd) {
    if (!g_engine || !p_axomai_engine_load_url) return;

    RECT rc;
    GetClientRect(hWnd, &rc);
    float viewport_w = (float)(rc.right - rc.left);
    float viewport_h = (float)(rc.bottom - rc.top - 50);

    if (url_str.rfind("data:text/html,", 0) == 0) {
        std::string html = url_str.substr(15);
        p_axomai_engine_load_html(g_engine, html.c_str(), viewport_w, viewport_h);
    } else {
        p_axomai_engine_load_url(g_engine, url_str.c_str(), viewport_w, viewport_h);
    }

    if (p_axomai_engine_get_max_scroll) {
        g_max_scroll_y = p_axomai_engine_get_max_scroll(g_engine);
    }
    g_scroll_y = 0.0f;

    std::wstring wurl(url_str.begin(), url_str.end());
    SetWindowTextW(g_hAddrEntry, wurl.c_str());

    InvalidateRect(hWnd, NULL, FALSE);
}

void RenderCanvas(HDC hdc, HWND hWnd) {
    RECT rc;
    GetClientRect(hWnd, &rc);
    int width = rc.right - rc.left;
    int height = rc.bottom - rc.top;

    Bitmap backBuffer(width, height);
    Graphics g(&backBuffer);
    g.SetSmoothingMode(SmoothingModeAntiAlias);

    SolidBrush bgBrush(Color(255, 255, 255, 255));
    g.FillRectangle(&bgBrush, 0, 0, width, height);

    if (g_engine && p_axomai_engine_get_display_count && p_axomai_engine_get_display_command) {
        size_t count = p_axomai_engine_get_display_count(g_engine);
        for (size_t i = 0; i < count; i++) {
            FFIDisplayCommand cmd;
            if (p_axomai_engine_get_display_command(g_engine, i, &cmd)) {
                float render_y = cmd.y - g_scroll_y + 50.0f; // offset below toolbar

                if (cmd.cmd_type == 0) { // DrawRect
                    Color c = ParseHexColor(cmd.color);
                    SolidBrush b(c);
                    g.FillRectangle(&b, cmd.x, render_y, cmd.width, cmd.height);
                } else if (cmd.cmd_type == 1) { // DrawText
                    Color c = ParseHexColor(cmd.color);
                    SolidBrush b(c);
                    int style = FontStyleRegular;
                    if (cmd.font_weight_bold) style |= FontStyleBold;
                    if (cmd.font_style_italic) style |= FontStyleItalic;

                    Font font(L"Segoe UI", cmd.font_size, style, UnitPixel);
                    std::wstring wtext;
                    int len = MultiByteToWideChar(CP_UTF8, 0, cmd.text, -1, NULL, 0);
                    if (len > 0) {
                        wtext.resize(len);
                        MultiByteToWideChar(CP_UTF8, 0, cmd.text, -1, &wtext[0], len);
                    }
                    PointF pt(cmd.x, render_y);
                    g.DrawString(wtext.c_str(), -1, &font, pt, &b);
                } else if (cmd.cmd_type == 3) { // DrawInput
                    Color borderC = cmd.is_focused ? Color(255, 26, 115, 232) : Color(255, 223, 225, 229);
                    Color fillC = cmd.is_focused ? Color(255, 255, 255, 255) : Color(255, 248, 249, 250);
                    Pen pen(borderC, cmd.is_focused ? 2.0f : 1.0f);
                    SolidBrush fillB(fillC);

                    g.FillRectangle(&fillB, cmd.x, render_y, cmd.width, cmd.height);
                    g.DrawRectangle(&pen, cmd.x, render_y, cmd.width, cmd.height);

                    std::wstring wval;
                    int len = MultiByteToWideChar(CP_UTF8, 0, cmd.text, -1, NULL, 0);
                    if (len > 0) {
                        wval.resize(len);
                        MultiByteToWideChar(CP_UTF8, 0, cmd.text, -1, &wval[0], len);
                    }
                    if (cmd.is_focused) wval += L"|";

                    SolidBrush textB(Color(255, 32, 33, 36));
                    Font font(L"Segoe UI", 12.0f, FontStyleRegular, UnitPixel);
                    PointF pt(cmd.x + 12.0f, render_y + (cmd.height / 4.0f));
                    g.DrawString(wval.c_str(), -1, &font, pt, &textB);
                } else if (cmd.cmd_type == 4) { // DrawButton
                    SolidBrush b(Color(255, 26, 115, 232));
                    g.FillRectangle(&b, cmd.x, render_y, cmd.width, cmd.height);

                    std::wstring wlbl;
                    int len = MultiByteToWideChar(CP_UTF8, 0, cmd.text, -1, NULL, 0);
                    if (len > 0) {
                        wlbl.resize(len);
                        MultiByteToWideChar(CP_UTF8, 0, cmd.text, -1, &wlbl[0], len);
                    }
                    SolidBrush textB(Color(255, 255, 255, 255));
                    Font font(L"Segoe UI", 12.0f, FontStyleBold, UnitPixel);
                    PointF pt(cmd.x + (cmd.width / 4.0f), render_y + (cmd.height / 4.0f));
                    g.DrawString(wlbl.c_str(), -1, &font, pt, &textB);
                }
            }
        }
    }

    Graphics windowGraphics(hdc);
    windowGraphics.DrawImage(&backBuffer, 0, 0);
}

LRESULT CALLBACK WndProc(HWND hWnd, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
    case WM_CREATE: {
        // Toolbar UI Controls
        CreateWindowW(L"BUTTON", L"◄", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 6, 6, 36, 32, hWnd, (HMENU)101, NULL, NULL);
        CreateWindowW(L"BUTTON", L"►", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 46, 6, 36, 32, hWnd, (HMENU)102, NULL, NULL);
        CreateWindowW(L"BUTTON", L"↻", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 86, 6, 36, 32, hWnd, (HMENU)103, NULL, NULL);
        CreateWindowW(L"BUTTON", L"🏠", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 126, 6, 36, 32, hWnd, (HMENU)104, NULL, NULL);

        g_hAddrEntry = CreateWindowW(L"EDIT", L"", WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL, 168, 8, 680, 28, hWnd, (HMENU)105, NULL, NULL);
        CreateWindowW(L"BUTTON", L"Go", WS_VISIBLE | WS_CHILD | BS_DEFPUSHBUTTON, 854, 6, 60, 32, hWnd, (HMENU)106, NULL, NULL);

        // Start 60 FPS Event Loop timer for async fetch, microtasks, and JS timers
        SetTimer(hWnd, 1, 16, NULL);
        break;
    }

    case WM_TIMER: {
        if (wParam == 1 && g_engine && p_axomai_engine_process_event_loop) {
            RECT rc;
            GetClientRect(hWnd, &rc);
            float viewport_w = (float)(rc.right - rc.left);
            float viewport_h = (float)(rc.bottom - rc.top - 50);
            if (p_axomai_engine_process_event_loop(g_engine, viewport_w, viewport_h)) {
                if (p_axomai_engine_get_max_scroll) {
                    g_max_scroll_y = p_axomai_engine_get_max_scroll(g_engine);
                }
                InvalidateRect(hWnd, NULL, FALSE);
            }
        }
        break;
    }

    case WM_COMMAND: {
        int wmId = LOWORD(wParam);
        if (wmId == 106) { // Go button
            wchar_t buf[1024];
            GetWindowTextW(g_hAddrEntry, buf, 1024);
            char ubuf[1024];
            WideCharToMultiByte(CP_UTF8, 0, buf, -1, ubuf, 1024, NULL, NULL);
            LoadUrlInEngine(ubuf, hWnd);
        } else if (wmId == 104) { // Home
            std::string homeUrl = std::string("data:text/html,") + DEFAULT_PAGE_HTML;
            LoadUrlInEngine(homeUrl, hWnd);
        }
        break;
    }

    case WM_LBUTTONDOWN: {
        float x = (float)LOWORD(lParam);
        float y = (float)HIWORD(lParam) - 50.0f; // offset toolbar height
        if (y >= 0 && g_engine && p_axomai_engine_handle_click) {
            char out_url[1024] = {0};
            if (p_axomai_engine_handle_click(g_engine, x, y, g_scroll_y, out_url, 1024)) {
                if (out_url[0] != '\0') {
                    LoadUrlInEngine(out_url, hWnd);
                }
            }
            InvalidateRect(hWnd, NULL, FALSE);
        }
        break;
    }

    case WM_CHAR: {
        if (g_engine && p_axomai_engine_handle_key) {
            char key_str[2] = {(char)wParam, 0};
            char out_url[1024] = {0};
            if (p_axomai_engine_handle_key(g_engine, key_str, out_url, 1024)) {
                if (out_url[0] != '\0') {
                    LoadUrlInEngine(out_url, hWnd);
                }
            }
            InvalidateRect(hWnd, NULL, FALSE);
        }
        break;
    }

    case WM_MOUSEWHEEL: {
        short delta = GET_WHEEL_DELTA_WPARAM(wParam);
        g_scroll_y -= (float)delta / 2.0f;
        if (g_scroll_y < 0.0f) g_scroll_y = 0.0f;
        if (g_scroll_y > g_max_scroll_y) g_scroll_y = g_max_scroll_y;
        InvalidateRect(hWnd, NULL, FALSE);
        break;
    }

    case WM_PAINT: {
        PAINTSTRUCT ps;
        HDC hdc = BeginPaint(hWnd, &ps);
        RenderCanvas(hdc, hWnd);
        EndPaint(hWnd, &ps);
        break;
    }

    case WM_DESTROY:
        KillTimer(hWnd, 1);
        PostQuitMessage(0);
        break;

    default:
        return DefWindowProc(hWnd, message, wParam, lParam);
    }
    return 0;
}

int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    GdiplusStartupInput gdiplusStartupInput;
    ULONG_PTR gdiplusToken;
    GdiplusStartup(&gdiplusToken, &gdiplusStartupInput, NULL);

    if (!LoadEngineDll()) {
        MessageBoxW(NULL, L"Failed to load Rust engine DLL (axomai_engine.dll)!", L"Axomai Engine Error", MB_ICONERROR);
        return 1;
    }

    g_engine = p_axomai_engine_create();

    WNDCLASSEXW wcex = { sizeof(WNDCLASSEX) };
    wcex.style = CS_HREDRAW | CS_VREDRAW;
    wcex.lpfnWndProc = WndProc;
    wcex.hInstance = hInstance;
    wcex.hCursor = LoadCursor(NULL, IDC_ARROW);
    wcex.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1);
    wcex.lpszClassName = L"AxomaiBrowserCPPClass";

    RegisterClassExW(&wcex);

    HWND hWnd = CreateWindowW(L"AxomaiBrowserCPPClass", L"Axomai Browser - C++ Native GUI",
        WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, 960, 680, NULL, NULL, hInstance, NULL);

    if (!hWnd) return FALSE;

    ShowWindow(hWnd, nCmdShow);
    UpdateWindow(hWnd);

    // Initial load
    std::string homeUrl = std::string("data:text/html,") + DEFAULT_PAGE_HTML;
    LoadUrlInEngine(homeUrl, hWnd);

    MSG msg;
    while (GetMessage(&msg, NULL, 0, 0)) {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }

    if (g_engine && p_axomai_engine_free) {
        p_axomai_engine_free(g_engine);
    }

    GdiplusShutdown(gdiplusToken);
    return (int)msg.wParam;
}
