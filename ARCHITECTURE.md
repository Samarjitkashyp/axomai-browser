# 🏛️ Axomai Browser Engine — Architectural Whitepaper

## 1. High-Level System Architecture

Axomai Browser is designed around a modern **Multi-Process Sandboxed Architecture** in Rust, ensuring that untrusted web content cannot compromise system integrity, crash the browser UI, or access unauthorized hardware resources.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        BROWSER MAIN PROCESS                            │
│  - Window Management (tao / wry)     - Tab & Workspace Management     │
│  - Settings & Profile Database       - IPC Message Dispatch Bus        │
│  - Native Auto-Updater Engine        - Heritage Theme Orchestrator     │
└──────────────┬──────────────────────────────────────────┬──────────────┘
               │                                          │
               ▼ (IPC / Sandboxed Pipe)                   ▼ (IPC / Sandboxed Pipe)
┌──────────────────────────────┐        ┌──────────────────────────────┐
│       NETWORK PROCESS        │        │   SANDBOXED RENDER PROCESS   │
│  - HTTP/3, TLS 1.3, DoH      │        │  - HTML5 Tree Builder        │
│  - HttpCache & ETag Engine   │        │  - CSSOM & 2D Grid / Flexbox │
│  - EasyList AdBlock Shield   │        │  - Google V8 & WASM Stack VM │
│  - Multi-Thread Downloader   │        │  - HarfBuzz Indic Shaper     │
│  - WebRTC Peer Signaling     │        │  - Web Crypto & Web Audio    │
└──────────────┬───────────────┘        └──────────────┬───────────────┘
               │                                       │
               │ (Decoded Bitmaps / Frames)            │ (DisplayList & Quads)
               ▼                                       ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        GPU COMPOSITOR PROCESS                          │
│  - LayerTree Decomposition (Transform, Scroll, Fixed, Sticky)          │
│  - Damage Region Tracking & Partial Repaint                            │
│  - WebGPU (WGSL) & WebGL 2.0 State Machine Pipeline                    │
│  - Direct Surface Presentation to Screen                               │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Core Subsystem Highlights

### A. Google V8 & WebAssembly (WASM) VM
* Pure Google V8 isolate integration for ECMAScript standard compliance.
* Standalone W3C WebAssembly 1.0/2.0 binary parser (`\0asm\1`), LEB128 decoder, and stack virtual machine supporting numeric opcodes and linear memory growth.

### B. Hardware Graphics & Compositing (WebGPU / WebGL 2.0)
* High-performance `LayerTree` decomposition allocating independent compositing layers for transforms, opacity, and sticky elements.
* Direct GPU command buffers executing vertex and quad shaders with `DamageRegion` bounding box tracking to minimize CPU/GPU overdraw.

### C. HarfBuzz Indic & Complex Script Shaper
* Native OpenType GSUB/GPOS lookup tables.
* Specialized support for **Assamese (অসমীয়া)**, Bengali, and Devanagari ligatures (য-ফলা, ৰ-কাৰ, যুক্তাক্ষৰ) and BiDi Arabic RTL paragraph shaping.

### D. Security & Multi-Process Sandbox
* Granular `SandboxPolicy` disabling raw socket creation and direct disk I/O inside renderer isolates.
* Content Security Policy (`CspPolicy`) directive parser blocking unauthorized scripts, styles, and fetch connections.

### E. AI Workspaces & Tab Hibernation
* Semantic URL categorization assigning tabs to dynamic workspaces (*"Coding"*, *"Research"*, *"Assam News"*).
* Background tab suspension dropping idle memory from 85MB to 2MB per tab (80%+ RAM reduction).

---

## 3. Platform Portability & Build Targets

| Target | Binary Format | Bridge Layer |
| :--- | :---: | :--- |
| **Windows 10 / 11** | `.exe` / `.msi` | Native Win32 / WebView2 / Tao |
| **Linux (Ubuntu, Fedora, Arch)** | `.deb` / `.rpm` / Flatpak | GTK3 / WebKitGTK / Wayland |
| **macOS (Apple Silicon & Intel)**| `.dmg` / `.app` | Cocoa / WebKit macOS |
| **Android Mobile** | `.apk` / `.aab` | Android NDK / JNI FFI (`android_bridge.rs`) |
