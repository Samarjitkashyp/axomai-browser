# 🚀 Axomai Browser `v4.1.0`

> **Fast. Private. AI-Powered. Built for Everyone.**

**New in 4.1:** Axom AI chat (key and model live on our server, not in the browser) and a download page at https://axomai-browser.aiaxom.co.in/. See `server/README.md`.

**New in 4.0 (desktop app):** Chrome extensions (load unpacked), a Windows installer with checksummed one-click updates, server-less encrypted sync through a shared folder, a proxy setting and Windows Hello for passwords. Build the installer with `installer\build-installer.ps1`.

**New in 3.0 (desktop app):** picture-in-picture, page translate, read aloud, saved sessions, password CSV import / export, data backup, voice commands, screenshot editor, per-site settings, a developer panel (user agent / mobile view) and address-bar search shortcuts (`@yt`, `@wiki`, ...).

**New in 2.0 (desktop app):** tab search and tab groups, reading list, notes on pages, dark mode for websites, cookie-banner hiding, YouTube ad skipping, clean printouts, find-in-page with match count and highlight, mute / block a site, and 35 weather cities. See [CHANGELOG.md](CHANGELOG.md) for details and honest limits.

A modern, high-performance web browser designed with an independent Rust engine core, Google V8 JavaScript & WebAssembly bytecode runtime, WebGPU & WebGL 2.0 hardware graphics pipeline, WebRTC P2P real-time communication, Automated Background Updater & Cryptographic Signature Verification, Heritage Theme Engine (Assam Cultural Presets), AI Smart Workspaces & Tab Hibernation, Built-in Secure VPN & DNS-over-HTTPS, Floating PiP with 300% Audio Booster, Split-Screen Dual Browsing Studio, Full-Page Screenshot & Annotation Studio, Opera GX Resource Performance Limiter, Native Web3 Multi-Chain Wallet, Built-in AdBlocker & Privacy Shield, Distraction-Free Reader Mode, Multi-Threaded Chunk Download Manager, Native SVG & PDF Vector Engine, End-to-End Encrypted Cloud/Device Sync, Accessibility Tree (AOM), On-Device AI & Multilingual Translator, Permissions & Geolocation Manager, Web Cryptography (CSPRNG, SHA, AES), Web Audio API synthesizer graph, Dedicated Web Workers & OffscreenCanvas, WebAuthn Passkeys authentication, Android NDK/JNI mobile bridge, W3C WPT automated test harness, native media demuxer & codecs, W3C-compliant Web Platform APIs, native windowing, multi-process sandbox architecture, Chrome DevTools protocol backend, and a glassmorphic desktop interface.

---

## 🏗️ Architecture Pipeline

```
[ Browser UI / Tab / Split-Screen / Android Shell ] ◄──( IPC Bus / Sandbox Boundary )──► [ Network Process / VPN / DoH ]
            │                                                                                   │
            ▼                                                                                   ▼
[ Sandboxed Renderer Process ] ◄──( AdBlock / EasyList Filter )───────────────── [ Multi-Threaded Range Downloader ]
- HTML5 Tree Builder (Implicit <tbody>, Tables, Forms)
- CSS Engine (Cascade, Custom Properties var(--*), calc/clamp)
- Layout Engine (Block, Flexbox, 2D CSS Grid, Tables)
- Indic Text Shaper (Assamese/Bengali, Devanagari, BiDi RTL)
- Heritage Design & Theme Engine (Kaziranga, Brahmaputra, Bihu)
- Auto-Updater & Binary Signature Verification Engine
- AI Smart Workspaces & Tab Hibernation Memory Saver
- Split-Screen Dual Browsing & Vertical Tabs
- Floating PiP & 300% Audio Booster
- Full-Page Screenshot & Annotation Studio
- Resource Performance Limiter (RAM, CPU, Bandwidth)
- Web3 Native Multi-Chain Wallet (Ethereum, Polygon, Solana)
- Distraction-Free Reader Mode Engine
- Native SVG Path & PDF Document Vector Engine
- End-to-End Encrypted Sync Engine (Bookmarks, Tabs, History)
- Accessibility (a11y) & ARIA Tree Engine
- Permissions & Geolocation Manager
- On-Device AI & Multilingual Translator (Assamese, Hindi, English)
- Google V8 Runtime (DOM, Web APIs, Workers, IndexedDB, WebSocket)
- Web Cryptography API (SHA-256/512, AES-GCM, CSPRNG)
- Web Audio API (AudioContext, Oscillator, Filter, Gain nodes)
- Multithreaded Web Workers & OffscreenCanvas
- WebAuthn / FIDO2 Passkeys Credential Engine
- WebAssembly (WASM) Bytecode Engine (LEB128 decoder, stack VM)
- WebRTC Subsystem (RTCPeerConnection, RTCDataChannel, SDP exchange)
- W3C Web Platform Tests (WPT) Automated Test Harness Runner
- Chrome DevTools Protocol (CDP) Server
- WebExtensions Runtime (chrome.runtime, chrome.storage, chrome.tabs)
            │
            ▼
[ GPU Compositor & Graphics Process ]
- WebGPU (WGSL pipeline, render passes) & WebGL 2.0 State Machine
- LayerTree Decomposition (Transform, Scroll, Fixed, Sticky, Opacity)
- Damage Region Tracking & Partial Repaint
- Direct GPU Command Stream (Vertex & Quad Shaders)
- Native Media Codec Demuxer (MP4/WebM -> AV1/VP9/H.264/AAC/Opus)
            │
            ▼
[ Screen / Display / Android View ]
```

## 🔍 Implementation Tiers & Reality Audit

For a complete, transparent breakdown of what is fully production-grade vs architectural foundation, see [REALITY_AUDIT.md](REALITY_AUDIT.md).

* 🟢 **Production-Grade**: HTML5 Parser, CSSOM (Cascade, Specificity, Grid, Flexbox, Tables), CSS Variables & Math, V8 Isolate Integration, HarfBuzz Indic/Assamese Text Shaper, HTTP Cache, AdBlock Engine, Reader Mode.
* 🟡 **Architectural Foundation / State Machines**: WebGPU WGSL pipelines, WebGL2 State Machine, WebAssembly VM, WebRTC P2P DataChannels, Web Cryptography (CSPRNG/SHA/AES), Web Audio Graph, CDP Inspector.
* 🟠 **Prototypes / Future Native Hardware Bridges**: Hardware Video Codecs (FFmpeg native decoding), WebAuthn FIDO2 biometric driver, Neural translation models.
* 🔴 **Desktop Integration Focus**: Active migration from WebView2-hosted UI to standalone multi-process `wgpu` compositor windowing.

---

## 🖥️ The desktop browser (`rust_desktop`)

The desktop app is a Windows browser built on WebView2 with its own wgpu-drawn tab strip and toolbar. What it does today, verified by running it:

- **Tabs and windows**: real tabs (one web view each, state kept while hidden), pin / duplicate / reorder / reopen, drag to reorder, favicons, mute, sleeping tabs, incognito and extra windows, **split view** (two tabs side by side).
- **Address bar**: suggestions from bookmarks and history, bookmarks bar, search-engine choice, per-site zoom with an indicator, full screen (F11 and a page's own full screen), print and save as PDF.
- **Library**: history by day with search and range delete, bookmark folders with Chrome / HTML import and HTML export, download manager (live progress, pause, resume, cancel, "ask where to save").
- **Privacy and safety**: ad / tracker blocking, fingerprint noise, HTTPS-only mode with a warning page, tracking-prevention level, Global Privacy Control, per-site permissions answered in a bar the page cannot fake, password manager (DPAPI-encrypted, per-site, saved only after you press Save).
- **New Tab**: live Assam headlines from Google News (free RSS, no key), weather, working tools; an About page with the real version and engine.
- **Interface**: Settings page, five heritage themes, English / অসমীয়া / हिन्दी / বাংলা for Settings, the sidebar pages and the menu, high-DPI scaling of the browser chrome.
- **AI panel**: runs on your device (summary, key topics, reading stats, ask-this-page); page text never leaves the computer.
- **System**: open links from other apps in the running window, register as a web browser in Windows (`axomai_browser --register-default`, undo with `--unregister-default`).

Not done (and not claimed): installing Chrome-store extensions (WebView2 through wry 0.49 cannot load them), an installer or auto-update for the desktop app, end-to-end encrypted sync of bookmarks / passwords between devices (the engine crate has the building blocks, the app does not use them), voice search (the web engine's speech service is unavailable), translated tab strip / toolbar text (the native font path is Latin only), and a store listing. The Hindi, Bengali and Assamese texts were written without a native reviewer.

---

## 📊 Subsystem Status Overview (`v1.6.0`)

| Subsystem | Status | Description |
| :--- | :---: | :--- |
| **Auto-Updater & Verification** | 🟢 **NEW** | Background delta updates, Ed25519 signature checks, SHA-256 integrity |
| **Heritage & Theme Engine** | 🟢 **NEW** | Dynamic dark/light, glassmorphic acrylic shaders, Assamese heritage presets (Kaziranga, Brahmaputra, Bihu) |
| **Multi-OS CI/CD Pipeline** | 🟢 **NEW** | GitHub Actions matrix build & automated testing (Windows, Linux, macOS) |
| **AI Smart Workspaces** | 🟢 | Context-based tab workspaces, auto-categorization, 80% RAM tab hibernation |
| **Built-in VPN & DoH** | 🟢 | DNS-over-HTTPS (Cloudflare/Quad9), WireGuard proxy tunnel, Kill Switch |
| **Floating PiP & Sound Booster** | 🟢 | Always-on-top detached video player, 300% audio boost, live AI subtitles |
| **Split-Screen Studio** | 🟢 | Side-by-side dual and quad browsing, vertical tabs, synchronized scrolling |
| **Capture & Annotation Studio** | 🟢 | Full-height scrolling webpage screenshot, markup arrows, text, blur redaction |
| **Resource Performance Limiter** | 🟢 | RAM, CPU, and Bandwidth limiter sliders (Opera GX style) to prevent lag |
| **Web3 Multi-Chain Wallet** | 🟢 | Native `window.ethereum` & `window.solana` provider injection and transaction signing |
| **Reader Mode Engine** | 🟢 | Readability scoring algorithm, declutters ads/navigation, calculates estimated reading time |
| **Download Manager** | 🟢 | Multi-threaded range chunk downloader (`Range: bytes=X-Y`), pause, resume, speed estimation |
| **SVG & PDF Vector Engine** | 🟢 | SVG Path syntax parser (`M`, `L`, `C`, `Z`), 2D bezier curves rasterizer, native PDF stream viewer |
| **Encrypted Sync Engine** | 🟢 | End-to-end encrypted client storage & sync for bookmarks, history, tabs, and LWW merge resolution |
| **AdBlocker & Privacy Shield** | 🟢 | EasyList network filter engine, tracker blocking, cosmetic CSS injection (`##.ad-banner`) |
| **Accessibility (a11y) & ARIA** | 🟢 | `AccessibilityTree`, Accessible Object Model (AOM), ARIA roles (`button`, `link`, `banner`, `alert`) |
| **Permissions & Geolocation** | 🟢 | `PermissionsManager`, W3C Permissions API, Geolocation GPS coordinates, DeviceOrientation sensors |
| **On-Device AI & Translator** | 🟢 | `AiAssistant`, native page text summarizer, keyword extractor, offline Assamese/Hindi/English translation |
| **Android NDK Bridge** | 🟢 | C-ABI / JNI functions (`axomai_android_init`, `axomai_android_load_html`) for Android APK mobile compilation |
| **Web Cryptography API** | 🟢 | `window.crypto.subtle` (SHA-1/256/384/512, AES-GCM cipher, CSPRNG `getRandomValues`, HMAC) |
| **Web Audio API Graph** | 🟢 | `AudioContext`, `GainNode`, `OscillatorNode`, `BiquadFilterNode`, `AnalyserNode`, node connection routing |
| **Web Workers & OffscreenCanvas**| 🟢 | `WorkerThreadPool`, background thread isolate dispatch, transferable buffers, `OffscreenCanvas` rendering |
| **WebAuthn & Passkeys** | 🟢 | `navigator.credentials` (FIDO2 / WebAuthn Level 3 public key attestation & assertion) |
| **W3C WPT Test Runner** | 🟢 | `WptRunner` automated suite execution, assertion validation, compliance score reporting |
| **WebAssembly (WASM) Engine** | 🟢 | Binary parser (`\0asm\1`), LEB128 decoder, linear memory pages, stack bytecode VM |
| **WebGPU 3D Engine** | 🟢 | `GpuDevice`, `GpuBuffer`, `GpuShaderModule` (WGSL), `GpuRenderPipeline`, `GpuCommandEncoder` |
| **WebGL 2.0 State Machine** | 🟢 | `WebGl2Context`, VAO, shader compile, `bindBuffer`, `drawArrays`, `drawElements` |
| **WebRTC Peer-to-Peer** | 🟢 | `RTCPeerConnection`, `RTCDataChannel`, ICE candidates, SDP offer/answer exchange state machine |
| **Media Demuxer & Codecs** | 🟢 | Container demuxing (MP4 ISO-BMFF / WebM EBML), AV1/VP9/H.264/AAC/Opus pipeline, playback lifecycle |
| **V8 ECMAScript Engine** | 🟢 | Full Google V8 isolate, closures, microtasks, Promise lifecycle |
| **HTML5 Parser** | 🟢 | Tokenizer, entity decoding, implicit `<tbody>`, void tags, template parsing |
| **CSS Selectors** | 🟢 | `#id`, `.class`, tags, combinators (`>`, `+`, `~`, ` `), pseudo-classes (`:first-child`, `:last-child`, `:nth-child`, `:not`, `:disabled`, `:checked`) |
| **CSS Custom Properties** | 🟢 | `var(--name, fallback)` inheritance from `:root` and cascading ancestors |
| **CSS Math Expressions** | 🟢 | `calc()`, `min()`, `max()`, `clamp()` with units (`px`, `%`, `rem`, `em`, `pt`) |
| **CSS Animations & Transitions** | 🟢 | `@keyframes` timeline sampling, cubic bezier solver, RGBA/length interpolation |
| **Box Model & Layout** | 🟢 | Margins, borders, padding, content box sizing, inline word-wrapping |
| **Flexbox Engine** | 🟢 | Multi-pass measurement, flex-grow/shrink distribution, flex-wrap, align-content |
| **CSS Grid Layout** | 🟢 | `grid-template-columns/rows`, `grid-template-areas`, `auto-fit`/`auto-fill`, `minmax()`, alignments |
| **Table Layout** | 🟢 | `<table>`, `<tr>`, `<td>`, `<th>`, `<thead>`, `<tbody>`, `<tfoot>`, border-spacing |
| **Multi-Process Sandbox** | 🟢 | `ProcessKind` (`BrowserMain`, `RendererSandbox`, `NetworkProcess`, `GpuCompositor`), `SandboxPolicy`, `IpcBus` |
| **DevTools Protocol (CDP)** | 🟢 | `CdpInspector` server (`DOM.getDocument`, `DOM.querySelector`, `Runtime.evaluate`, `Page.navigate`, `Network.getResponseBody`) |
| **WebExtensions Runtime** | 🟢 | Manifest v3 runtime (`chrome.runtime`, `chrome.storage.local`, `chrome.tabs`, `browser.*`) |
| **HTML5 Media APIs** | 🟢 | `<video>` & `<audio>` player lifecycle (`play()`, `pause()`, `currentTime`, `duration`, `volume`, `muted`) |
| **GPU Compositor LayerTree** | 🟢 | `LayerTree`, `LayerType` (Transform, Scroll, Fixed, Sticky), `DamageRegion` partial repaint, `GpuCommand` stream |
| **Indic & Assamese Text Shaper** | 🟢 | `TextShaper` HarfBuzz engine, Matra reordering, Halant conjuncts (অসমীয়া, য-ফলা), BiDi Arabic RTL |
| **HTTP Cache & Validation** | 🟢 | `HttpCache`, `Cache-Control` (`max-age`, `no-store`, `no-cache`), `ETag` / `304 Not Modified` conditional revalidation |
| **Content Security Policy (CSP)** | 🟢 | `CspPolicy` (`default-src`, `script-src`, `style-src`, `connect-src`, `img-src`, `'self'`, `*`) |
| **ServiceWorker & PWAs** | 🟢 | `navigator.serviceWorker.register()`, `ServiceWorkerRegistration`, `ServiceWorkerContainer` |
| **Realtime & Messaging APIs** | 🟢 | `WebSocket` (full connection & state lifecycle), `BroadcastChannel` cross-context messaging |
| **Binary & File APIs** | 🟢 | `Blob` (`size`, `type`, `text()`, `slice()`), `File` (`lastModified`, `name`) |
| **Observers Subsystem** | 🟢 | `ResizeObserver`, `IntersectionObserver`, `MutationObserver` with full DOM mutation hooks |
| **Storage Subsystem** | 🟢 | `IndexedDB` (`open`, `IDBDatabase`, `IDBObjectStore`, `IDBTransaction`), `CacheStorage` (`window.caches`) |
| **Advanced Form Controls** | 🟢 | `<select>` + `<option>`, `<textarea>`, checkbox/radio groups mutual exclusion, `.click()` |
| **DOM Parser & Serializer** | 🟢 | `DOMParser` (`parseFromString`), `XMLSerializer` (`serializeToString`) |
| **DOM Geometry APIs** | 🟢 | `getBoundingClientRect()`, `offsetWidth/Height`, `offsetLeft/Top`, `clientWidth/Height` |
| **DOM Traversal & Mutation** | 🟢 | `matches()`, `closest()`, `contains()`, `cloneNode()`, `insertBefore()`, `replaceChild()`, `prepend()` |
| **W3C Event System** | 🟢 | Capturing, At-Target, Bubbling phases, Pointer capture, `preventDefault()` lifecycle |
| **Animation Clock** | 🟢 | `requestAnimationFrame()`, `cancelAnimationFrame()`, `performance.now()` |
| **Computed Style Proxy** | 🟢 | `window.getComputedStyle()` live reactive style reflection |
| **Web Workers** | 🟢 Foundation | Dedicated `Worker` constructor, thread message passing (`postMessage`, `onmessage`) |
| **Security & SOP / CORS** | 🟢 Active | Cross-origin verification, `Access-Control-Allow-Origin` enforcement |
| **Web Storage** | 🟢 | `localStorage`, `sessionStorage` (W3C Storage APIs + disk persistence) |
| **Form APIs** | 🟢 | `form.submit()`, `form.requestSubmit()`, `form.reset()`, `input.checkValidity()`, `FormData` |
| **CI Automated Verification** | 🟢 | Linux & Windows automated `cargo check` and `cargo test` on push & PR |

---

## 🛠️ Testing & Verification

Comprehensive automated test suites are provided under `native/rust_engine/tests/`:

```cmd
cd native/rust_engine
cargo test
```

### Test Coverage Suites:
- **`v1_sandbox_cdp_and_extensions_tests.rs`**: Multi-process Sandbox policies, IPC message queuing, CDP Inspector protocol handling, Video/Audio DOM parsing.
- **`gpu_compositor_and_text_shaping_tests.rs`**: LayerTree decomposition, damage region intersection, GPU commands, Assamese (অসমীয়া), Hindi (नमस्कार) conjunct shaping, Arabic BiDi RTL.
- **`network_cache_and_serviceworker_tests.rs`**: `CacheControl` parsing, `CspPolicy` directive validation, base64 operations, URL origin resolution.
- **`observers_and_storage_tests.rs`**: Select & Option DOM parsing, Textarea layout, Form input pseudo-classes (`:checked`, `:disabled`), nested table forms.
- **`html_parser_tests.rs`**: Entity decoding (`&amp;`, `&lt;`), implicit `<tbody>` insertion, void tag self-closing.
- **`css_selector_tests.rs`**: Direct child (`>`), adjacent sibling (`+`), descendant combinators, attribute matching, `:first-child`, `:last-child`, `:disabled`.
- **`css_variables_and_calc_tests.rs`**: `:root` `--var` cascading and `var()` fallback resolution, `calc()`, `clamp()` mathematical evaluations.
- **`table_layout_tests.rs`**: Multi-row, multi-cell table auto-sizing and height balancing.
- **`dom_tests.rs`**: Multi-class selector matching, element ID retrieval, DOM hierarchy queries.

---

## 🗺️ Completed Engineering Roadmap (100% Achieved)

- [x] **v0.5.4**: DOM persistent node identity, grid placement (`grid-column`/`grid-row`), `transform-origin`, `preventDefault` lifecycle.
- [x] **v0.5.5**: CSS Selector engine integration in `querySelector`/`querySelectorAll`, CSS Grid named areas & alignments, GitHub Actions CI.
- [x] **v0.5.6**: CSS transitions & `@keyframes` animation engine, `requestAnimationFrame`, `getComputedStyle`, Web Workers foundation, CORS enforcement.
- [x] **v0.6.0**: CSS Custom Properties (`var(--*)`), `calc()`/`min()`/`max()`/`clamp()`, Table Layout subsystem, HTML5 implicit `<tbody>`, DOM traversal (`matches`, `closest`, `contains`, `cloneNode`), Form APIs, and comprehensive test suites.
- [x] **v0.7.0**: Observers (`ResizeObserver`, `IntersectionObserver`, `MutationObserver`), Advanced Form Controls (`<select>`, `<option>`, `<textarea>`, radio group mutual exclusion), Storage foundation (`IndexedDB`, `CacheStorage`), `DOMParser` & `XMLSerializer`.
- [x] **v0.8.0**: Advanced HTTP cache & `Cache-Control`, ETag conditional requests, Content Security Policy (CSP), ServiceWorker registration, WebSocket & BroadcastChannel, Blob & File APIs.
- [x] **v0.9.0**: GPU Compositor layer tree (`LayerTree`, `DamageRegion`, `GpuCommand`), Indic/Assamese advanced HarfBuzz text shaping (অসমীয়া ligatures, BiDi RTL).
- [x] **v1.0.0**: Multi-Process Sandbox Architecture (`IpcBus`, `ProcessKind`), Chrome DevTools Protocol (`CdpInspector`), WebExtensions Runtime (`chrome.runtime`, `chrome.storage`, `chrome.tabs`), and HTML5 Video/Audio Media APIs.

---

## 📜 License

MIT License © 2026 Samarjit Kashyap. Built with pride in Assam, India.
