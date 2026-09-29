# 🚀 Axomai Browser

A modern web browser built **100% natively in Rust & C++**, implementing every core browser subsystem from TCP sockets to GPU screen pixels.

- **Core Engine (Rust)**: High-performance HTML5 DOM Parser, CSS3 Cascade Engine, Box Model Layout Calculator, JS Engine (`console.log`, `document.write`), and Display List Painter.
- **GUI Application (C++)**: Native Win32 GDI+ Windowing UI with double-buffered rendering canvas, Chrome address bar, navigation history, and interactive event handling.

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
| **Network Layer** | Rust (`network.rs`) | URL parsing (`http`, `https`, `file`, `data`), TCP socket creation, SSL wrapping via `ureq`, HTTP GET formatting & response handling. |
| **HTML Parser** | Rust (`html_parser.rs`) | Tokenization, HTML entity decoding, DOM Node structures (`Element`, `Text`), and stack-based tree construction with auto-closing tag recovery. |
| **CSS Engine** | Rust (`css_parser.rs`) | Selector parsing (`tag`, `.class`, `#id`, `descendant`), declaration parsing, UA stylesheets, author rules cascade, and style inheritance. |
| **Layout Engine** | Rust (`layout.rs`) | Box model calculations, block vertical stacking, inline text word-wrapping, and pill geometry measurement. |
| **JS Engine** | Rust (`js_engine.rs`) | Minimal JS execution engine evaluating `console.log`, `document.write`, `var`/`let`/`const`, string concatenation, and DOM tree mutation. |
| **Painter Engine** | Rust (`painter.rs`) | Walk layout tree in document order, emit `DrawRect`, `DrawText`, `DrawInput`, `DrawButton` display commands over C-FFI ABI. |
| **Browser GUI** | C++ (`cpp_gui/main.cpp`) | Win32 Window, address bar navigation, Back/Forward browser history, status updates, GDI+ canvas rendering, and mouse/keyboard interaction. |

---

## 🛠️ How to Build & Run

### 1. Build Entire Project
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
- [x] **JS Engine** (`console.log`, `document.write` DOM tree mutations).
- [x] **C++ Win32 GDI+ Painter** with double buffering, scroll & mouse wheel support.
- [x] **Navigation History** (Back / Forward).

---

## 📜 License
MIT License - Created for educational and high-performance browser architecture learning.
