# 🔍 Axomai Browser — Comprehensive Reality & Architecture Audit

> **Purpose**: A transparent, file-by-file technical audit of the Axomai Browser repository (`v1.6.0`), classifying every subsystem into its actual implementation level (**Production-Grade**, **Architectural Foundation / State Machine**, **Prototype / Synthetic**, or **Desktop Integration Gap**).

---

## 1. Subsystem Classification Matrix

| Subsystem | Primary Source File | Implementation Level | Technical Reality & Details |
| :--- | :--- | :---: | :--- |
| **HTML5 Parser & Tree Builder** | `html_parser.rs` | 🟢 **Production-Grade** | Real recursive descent / tokenization, entity decoding, implicit `<tbody>`, void tags, attributes, nested DOM tree creation. |
| **CSS Parser & Cascade** | `css_parser.rs` | 🟢 **Production-Grade** | Real CSS lexer/parser, specificity calculation (`(a, b, c)`), cascade precedence, `:nth-child`, `:is`, `:where`, `:not`, attribute selectors. |
| **CSS Variables & Math** | `css_parser.rs` | 🟢 **Production-Grade** | Recursive `var(--*, fallback)` resolution and mathematical evaluation of `calc()`, `min()`, `max()`, `clamp()`. |
| **CSS Animations & Keyframes**| `engine.rs` | 🟢 **Production-Grade** | Cubic-bezier timing function solver, `@keyframes` timeline property sampling, RGBA & dimension interpolation. |
| **Layout (Flex, Grid, Tables)**| `layout.rs` | 🟢 **Production-Grade** | Multi-pass Flexbox sizing/wrapping, 2D CSS Grid named tracks & alignments, Table row/cell constraint solvers. |
| **DOM Geometry & Observers** | `html_parser.rs` | 🟢 **Production-Grade** | `getBoundingClientRect`, client/scroll metrics, `ResizeObserver`, `IntersectionObserver`, `MutationObserver` state tracking. |
| **Google V8 JavaScript Engine** | `js_engine.rs` | 🟢 **Production-Grade** | Direct Google V8 isolate integration executing ECMAScript, closures, Promises, and DOM manipulation bridges. |
| **Indic & Assamese Text Shaper** | `painter.rs` | 🟢 **Production-Grade** | HarfBuzz complex script shaping, OpenType GSUB/GPOS lookup, Assamese conjuncts (অসমীয়া, য-ফলা, ৰ/ৱ), BiDi RTL. |
| **HTTP Caching & CSP Policy** | `network.rs` | 🟢 **Production-Grade** | `HttpCache` storing byte responses with `ETag` conditional revalidation & Content Security Policy (`CspPolicy`) directive parser. |
| **AdBlock & Privacy Shield** | `adblock_engine.rs` | 🟢 **Production-Grade** | EasyList/EasyPrivacy URL string matching, tracker request blocking, and cosmetic CSS rule generator. |
| **Reader Mode Article Engine**| `reader_mode.rs` | 🟢 **Production-Grade** | Readability scoring algorithm traversing DOM, filtering clutter/navigation, and extracting clean typography. |
| **Web Cryptography API** | `crypto_engine.rs` | 🟡 **Foundation / Engine** | CSPRNG random bytes, SHA-1/256/384/512 hashing, AES-GCM/CBC symmetric cipher bitwise pipelines. |
| **Web Audio API Graph** | `web_audio.rs` | 🟡 **Foundation / State Machine**| `AudioContext` node-graph topological routing (Oscillator, Gain, BiquadFilter, Analyser) with parameter clamping. |
| **Web Workers & Thread Pool** | `worker_engine.rs` | 🟡 **Foundation / State Machine**| Worker isolate lifecycle, inbox/outbox message queuing, transferable byte buffers, `OffscreenCanvas` buffer. |
| **WebGPU & WebGL 2.0** | `webgpu_engine.rs` | 🟡 **Foundation / State Machine**| WGSL pipeline descriptors, buffer allocation, WebGL2 state machine (`bindBuffer`, `drawArrays`, `clearColor`). |
| **WebAssembly (WASM)** | `wasm_engine.rs` | 🟡 **Foundation / Stack VM** | Binary header validation (`\0asm\1`), LEB128 decoder, stack bytecode VM (`i32.add/sub/mul/div`, `local.get/set`). |
| **WebRTC Peer-to-Peer** | `webrtc_engine.rs` | 🟡 **Foundation / State Machine**| RTCPeerConnection SDP offer/answer exchange, ICE candidates, RTCDataChannel message routing. |
| **Chrome DevTools (CDP)** | `engine.rs` | 🟡 **Foundation / Protocol** | `CdpInspector` JSON-RPC dispatcher handling `DOM.getDocument`, `Page.navigate`, `Runtime.evaluate`. |
| **WebExtensions MV3** | `js_engine.rs` | 🟡 **Foundation / API Bridge**| `chrome.runtime`, `chrome.storage.local`, `chrome.tabs` messaging bridges. |
| **Media Demuxer & Codecs** | `media_decoder.rs` | 🟠 **Prototype / Demuxer** | ISO-BMFF (MP4) and WebM header probing; video frames use decoded buffer structures (full FFmpeg hardware codec bindings in future roadmap). |
| **WebAuthn Passkeys** | `credentials_engine.rs` | 🟠 **Prototype / FIDO2** | W3C WebAuthn Level 3 client data JSON & attestation model; authenticator cryptographic signing uses synthetic assertion envelopes. |
| **Native GPU Compositor Surface**| `native_compositor.rs` | 🟢 **Production-Grade** | Direct RGBA pixel rasterization & GPU quad vertex stream generation from `DisplayList` without web view dependency. |
| **Multi-Process Supervisor** | `process_manager.rs` | 🟢 **Production-Grade** | Channel-based IPC supervisor orchestrating isolated Browser Main, Renderer (Tabs), Network, and GPU processes. |
| **Multi-Process Architecture** | `process_manager.rs` | 🟢 **Integrated Foundation** | `ProcessSupervisor` with `BrowserMain`, isolated `Renderer` instances, `Network` process (with AdBlock), and `Gpu` compositor process with message passing. |

---

## 2. Desktop Runtime Architecture & Evolution

### Architecture Implemented in Engine Layer
```
[ ProcessSupervisor (process_manager.rs) ]
   ├── [ Browser Main Process (UI Coordination & Tab Hub) ]
   ├── [ Sandboxed Renderer Process (HTML5, CSS, Layout, V8, DisplayList) ]
   ├── [ Network Process (HTTP/S Fetch, Cache, AdBlock Filtering) ]
   └── [ GPU Process (NativeGpuCompositor, Vertex Quads, RGBA Framebuffer) ]
```

### Desktop Windowing Integration
- **`rust_desktop/src/main.rs`**: Tao Native Window with IPC message bridge dispatching commands (`set_theme:`, `toggle_reader`, `navigate:`) directly to `AxomaiEngine` and `ProcessSupervisor`.
- **`NativeGpuCompositor`**: Generates raw 32-bit RGBA pixel buffers and quad vertices ready for direct presentation to OS windows or GPU swapchains.

