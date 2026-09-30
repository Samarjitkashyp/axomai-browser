# 🚀 Axomai Browser `v0.3.0`

> **Fast. Private. AI-Powered. Built for Everyone.**

A modern web browser designed with high-performance Rust core, native windowing, and a rich, glassmorphic UI featuring workspaces, Axomai AI assistant, quick tools, and customizable new tab experience.

- **Modern Browser UI (`v0.3.0`)**: Pixel-perfect native browser shell with multi-tab management, omnibox, quick shortcuts, news feed, quick productivity tools, and dedicated Axomai AI sidebar.
- **Rust Desktop Shell (`rust_desktop/`)**: Native cross-platform desktop windowing with Tao & Wry (WebView2) with full DisplayList HTML5 canvas bridge.
- **Core Engine (`native/rust_engine/`)**: Partial HTML5-compatible DOM Parser, Partial CSS3 Cascade & Layout Engine, Google V8 ECMAScript runtime with asynchronous ScriptScheduler, and Display List Painter.
- **Native GUI Engine (`native/cpp_gui/`)**: Win32 GDI+ canvas rendering bridge with full image decoding and custom color parser.

---

## 🏗️ Architecture Pipeline

```
[ URL ] ──► ( 1. Network Layer - Rust ) ──► [ Raw HTML/HTTP ]
                                                   │
                                                   ▼
                                        ( 2. HTML DOM Parser - Rust )
                                                   │
                                                   ▼
                                           [ DOM AST Tree ]
                                                   │
                                                   ▼
                                         ( 3. CSS Engine - Rust )
                                                   │
                                                   ▼
                                        [ Styled DOM Tree ]
                                                   │
                                                   ▼
                                       ( 4. Layout Engine - Rust )
                                                   │
                                                   ▼
                                          [ Layout Box Tree ]
                                                   │
                                                   ▼
                                      ( 5. Painter Engine - Rust )
                                                   │
                                                   ▼
                                      [ C-FFI Display List Stream ]
                                                   │
                                                   ▼
                                     ( 6. C++ Win32 GDI+ Screen )
```

---

## 📦 Subsystems Overview

| Subsystem | Tech Stack | Responsibility |
| :--- | :--- | :--- |
| **Network Layer** | Rust (`network.rs`, `engine.rs`) | Non-blocking asynchronous page navigation via isolated per-engine channels, URL query & fragment resolution, origin calculation, TCP/TLS socket wrapping via `ureq`, and HTTP response handling. |
| **HTML Parser** | Rust (`html_parser.rs`) | **Partial HTML5-compatible parser**: Tokenization, HTML entity decoding, DOM Node structures (`Element`, `Text`), idempotent pointer registration (`register_dom_tree`), and stack-based tree construction. |
| **CSS Engine** | Rust (`css_parser.rs`) | **Partial CSS cascade/layout implementation**: Selector parsing (`tag`, `.class`, `#id`, `descendant`), declaration parsing, UA stylesheets, author rules cascade (`active_css_rules`), and dynamic restyling upon DOM mutations. |
| **Layout Engine** | Rust (`layout.rs`) | Box model calculations, block vertical stacking, inline text word-wrapping, and pill geometry measurement. |
| **JS Engine (V8)** | Rust + Google V8 (`js_engine.rs`, `engine.rs`) | **Google V8 ECMAScript Runtime**: Persistent Google V8 Isolate & Page Context with microtasks and closures.<br>**Async ScriptScheduler**: Non-blocking external `<script src="...">` fetching that never hangs HTML/CSS page rendering.<br>**Web Platform & Fetch APIs**: Asynchronous non-blocking `fetch()` with HTTP methods (`GET`, `POST`, `PUT`, `DELETE`), custom headers & bodies; W3C EventTarget implementation with true 3-phase dispatch (Capturing, At Target, Bubbling) + `eventPhase`, `once`, `passive`, `signal`; DOM bridge (`getElementById`, `querySelector`, `createElement`, `appendChild`, `removeChild`, `innerHTML`, `textContent`, `setAttribute`), and Timers (`setTimeout`, `setInterval`). |
| **Desktop Shell** | Rust (`rust_desktop/main.rs`) | Native Tao windowing & Wry host directly connected to `AxomaiEngine` via IPC bridge, 60 FPS event loop ticking, and HTML5 canvas display list rendering bridge. |
| **Painter & GUI** | Rust & C++ (`painter.rs`, `cpp_gui/`) | Walk layout tree in document order, emit display commands over C-FFI ABI, Win32 double-buffered GDI+ rendering (with in-memory bitmap stream decoding for images and CSS named/hex colors), and 60 FPS event loop timers. |

---

## 🛠️ How to Build & Run

### 1. Preview Modern Browser UI (Instant 1-Click)
Double-click `preview_ui.bat` or run:
```cmd
.\preview_ui.bat
```
*(Or open `ui/index.html` directly in any web browser).*

### 2. Run Native Rust Desktop Browser Shell
Compile and run the modern desktop shell with Tao + Wry (WebView2):
```cmd
.\run_rust_browser.bat
```
Or manually:
```cmd
cd rust_desktop
cargo run
```

### 3. Build Core Native Engine (Rust + C++ GDI+)
Run the automated build script to compile both the Rust Engine DLL and the C++ Native GUI application:
```cmd
.\build.bat
```

### 2. Run Native Axomai Browser App
```cmd
.\native\cpp_gui\axomai_browser.exe
```

### 3. Run Rust Unit Tests
To test the core browser engine in Rust:
```cmd
cd native\rust_engine
cargo test
```

---

## 🌐 Supported Features
- [x] **HTTP & HTTPS** networking with SSL/TLS support.
- [x] **Local `file://` & `data:` URLs**.
- [x] **Rust DOM Tree Construction** with stack error recovery.
- [x] **CSS Cascade & Inheritance** (`color`, `font-size`, `font-weight`, `margin`, `display`).
- [x] **Inline Text Wrapping & Block Box Model Layout**.
- [x] **Google V8 JavaScript Runtime** (ECMAScript, persistent isolate & page context, V8 microtasks checkpoint).
- [x] **V8 ↔ Rust DOM Bridge** (`document.write`, `getElementById`, `querySelector`, `createElement`, `appendChild`, `removeChild`, `remove`, `innerHTML`, `textContent`, `setAttribute`).
- [x] **Browser Web APIs & Networking** (`fetch()` via Rust `ureq`, `window`, `location`, `navigator`, `console`, `setTimeout`, `setInterval`).
- [x] **External Script Pipeline** (`<script src="...">` automatic URL resolution and HTTP fetching).
- [x] **W3C EventTarget System** (`window.addEventListener`, `document.addEventListener`, element event bubbling, preventDefault, and native click dispatching).
- [x] **C++ Win32 GDI+ Painter** with double buffering, scroll & mouse wheel support.
- [x] **Navigation History** (Back / Forward).

---

## 📜 License
MIT License - Created for educational and high-performance browser architecture learning.
