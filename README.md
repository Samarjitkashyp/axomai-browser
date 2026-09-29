# 🚀 Axomai Browser

A modern web browser built **100% from scratch in Python**, implementing every core browser subsystem from raw sockets to screen pixels.

Inspired by the canonical resource *Web Browser Engineering*, **Axomai Browser** demonstrates a clean, modular, senior-level browser pipeline architecture.

---

## 🏗️ Architecture Pipeline

Axomai Browser implements the full 5-stage browser pipeline:

```
[ URL ] ──► ( 1. Network Layer ) ──► [ Raw HTML/HTTP ]
                                             │
                                             ▼
                                    ( 2. HTML Parser )
                                             │
                                             ▼
                                      [ DOM Tree ]
                                             │
                                             ▼
                                    ( 3. CSS Engine )
                                             │
                                             ▼
                                   [ Styled DOM Tree ]
                                             │
                                             ▼
                                   ( 4. Layout Engine )
                                             │
                                             ▼
                                     [ Layout Tree ]
                                             │
                                             ▼
                                   ( 5. Painter Engine ) ──► [ Display Commands ] ──► ( Screen Pixels )
```

---

## 📦 Subsystems Overview

| Subsystem | File | Responsibility |
| :--- | :--- | :--- |
| **Network Layer** | [`browser/network.py`](browser/network.py) | URL parsing (`http`, `https`, `file`, `data`), TCP socket creation, SSL wrapping, HTTP GET request formatting, and response buffer handling. |
| **HTML Parser** | [`browser/html_parser.py`](browser/html_parser.py) | Tokenization, HTML entity decoding, DOM Node structures (`Element`, `Text`), and stack-based tree construction with auto-closing tag error recovery. |
| **CSS Engine** | [`browser/css_parser.py`](browser/css_parser.py) | Selector parsing (`tag`, `.class`, `#id`, `descendant`), declaration parsing, User-Agent stylesheets, author rules cascade, and style inheritance. |
| **Layout Engine** | [`browser/layout.py`](browser/layout.py) | Box model calculations, block vertical stacking, inline text word-wrapping, and font geometry measurement via Tkinter font metrics. |
| **Painter Engine** | [`browser/painter.py`](browser/painter.py) | Walk layout tree in document order, emit `DrawRect` and `DrawText` display commands, and paint on Tkinter Canvas. |
| **Browser GUI** | [`main.py`](main.py) | Address bar navigation, Back/Forward browser history, scroll events, status updates, and interactive main window. |

---

## 🛠️ How to Run

### 1. Launch Axomai Browser GUI
Run the main script to open the browser GUI window:
```bash
python main.py
```
Or open a specific website directly from the terminal:
```bash
python main.py https://example.org
```

### 2. Run Test Suite
To run all unit tests for networking, parsing, and styling:
```bash
python -m unittest discover tests
```

---

## 🌐 Supported Features
- [x] **HTTP & HTTPS** socket engine with SSL/TLS support.
- [x] **Local `file://` & `data:` URLs**.
- [x] **DOM Tree Construction** with stack error recovery.
- [x] **CSS Cascade & Inheritance** (`color`, `font-size`, `font-weight`, `margin`, `display`).
- [x] **Inline Text Wrapping & Block Box Model Layout**.
- [x] **Tkinter Canvas Painter** with scroll & mouse wheel support.
- [x] **Navigation History** (Back / Forward).

---

## 📜 License
MIT License - Created for educational and systems architecture learning.
