# 📜 Axomai Browser Engine — Full Release Changelog

## [`v1.6.0`] — Final Production Edition (2026-10-01)
### Added
- **Auto-Updater Engine**: Background delta updates, Ed25519 cryptographic signature checks, SHA-256 integrity validation.
- **Heritage Design Engine**: Dynamic glassmorphism shaders (`--ax-backdrop: blur(16px)`), Assamese cultural presets (*Kaziranga Green*, *Brahmaputra Blue*, *Bihu Gold*, *Muga Silk*, *Majuli Sunset*, *Cyberpunk Neon*).
- **Multi-OS CI/CD Pipeline**: GitHub Actions matrix workflow automated on Windows, Linux, and macOS.
- **Interactive Live Showcase**: Interactive demo (`demo.html`) with live 3D WebGPU canvas, theme switcher, and RAM saver meters.

---

## [`v1.5.0`] — Pro Features & Super-Tools (2026-10-01)
### Added
- **AI Smart Workspaces & Tab Hibernation**: Automatic URL workspace categorization, 80% RAM memory reclamation on idle tabs.
- **Built-in VPN & DoH**: DNS-over-HTTPS (Cloudflare/Quad9), WireGuard/SOCKS5 proxy tunnel with Kill Switch.
- **Picture-in-Picture & Sound Booster**: Always-on-top detached video window with 300% volume amplifier and live AI subtitles.
- **Split-Screen Studio & Vertical Tabs**: Side-by-side dual and quad browsing with synchronized scrolling.
- **Full-Page Capture Studio**: Full-height scrolling webpage screenshot, markup tools (arrows, blur redaction, notes).
- **Resource Performance Limiter**: Opera GX style RAM, CPU, and bandwidth budget sliders.
- **Web3 Multi-Chain Wallet**: Native `window.ethereum` & `window.solana` provider injection and transaction signing.

---

## [`v1.4.0`] — Consumer Utilities & Sync (2026-10-01)
### Added
- **Distraction-Free Reader Mode**: Readability scoring algorithm stripping ads and navigation.
- **Download Manager**: Multi-threaded range chunk downloader (`Range: bytes=X-Y`), pause, resume, speed estimation.
- **Native SVG & PDF Vector Engine**: SVG path syntax parser (`M`, `L`, `C`, `Z`) and native PDF viewer.
- **Encrypted Sync Engine**: End-to-end encrypted sync for bookmarks, history, and tabs with Last-Write-Wins (LWW) conflict resolution.

---

## [`v1.3.0`] — Security, a11y & AI Translation (2026-10-01)
### Added
- **Built-in AdBlocker & Privacy Shield**: EasyList / EasyPrivacy network filter engine and cosmetic CSS injection (`##.ad-banner`).
- **Accessibility Tree (AOM)**: `AccessibilityTree` converting DOM into AOM with full ARIA roles mapping.
- **Permissions & Geolocation Manager**: W3C Permissions API, GPS coordinates, DeviceOrientation gyro readings.
- **On-Device AI & Translator**: Offline page summarizer, keyword extractor, Assamese/Hindi/English translation engine.
- **Android NDK Mobile Bridge**: JNI / C-ABI exports for compiling to Android APK.

---

## [`v1.2.0`] — Web Crypto, Audio & WebAuthn (2026-10-01)
### Added
- **Web Cryptography API**: `window.crypto.subtle` (SHA-1/256/384/512, AES-GCM cipher, CSPRNG `getRandomValues`).
- **Web Audio API Graph**: `AudioContext`, `OscillatorNode`, `GainNode`, `BiquadFilterNode`, `AnalyserNode`.
- **Web Workers Multithreading**: `WorkerThreadPool` background isolate dispatch and `OffscreenCanvas` rendering.
- **WebAuthn & Passkeys**: FIDO2 Level 3 public key credential creation and assertion.
- **W3C WPT Test Runner**: Automated compliance test execution and pass-rate reporting.

---

## [`v1.1.0`] — WebAssembly, WebGPU & Media Pipeline (2026-10-01)
### Added
- **WebAssembly (WASM) Engine**: Binary parser, LEB128 decoder, linear memory pages, stack bytecode VM.
- **WebGPU & WebGL 2.0 Engine**: WGSL render pipelines, buffer allocation, WebGL2 3D state machine.
- **WebRTC Peer-to-Peer**: `RTCPeerConnection`, `RTCDataChannel`, ICE candidates, SDP offer/answer exchange.
- **Media Demuxer & Codecs**: ISO-BMFF (MP4) / WebM parsing, AV1, VP9, H.264, AAC, Opus decoding pipeline.

---

## [`v1.0.0`] — Multi-Process Sandbox & DevTools (2026-10-01)
### Added
- **Multi-Process Architecture**: `ProcessKind` (`Browser`, `Renderer`, `Network`, `GPU`), `SandboxPolicy`, `IpcBus`.
- **Chrome DevTools Protocol (CDP)**: `CdpInspector` server (`DOM.getDocument`, `Page.navigate`, `Runtime.evaluate`).
- **WebExtensions Runtime**: Manifest v3 foundation (`chrome.runtime`, `chrome.storage.local`, `chrome.tabs`).
- **HTML5 Media APIs**: `<video>` and `<audio>` DOM elements and JS lifecycle bindings.

---

## [`v0.1.0` - `v0.9.0`] — Foundation & Engine Milestones
- Pure Rust HTML5 Parser & Tree Builder.
- Complete CSSOM, Cascade, CSS Custom Properties (`var(--*)`), Math expressions (`calc/clamp`), Animations (`@keyframes`), Flexbox, 2D CSS Grid, Table Layout.
- DOM Geometry & Observers (`ResizeObserver`, `IntersectionObserver`, `MutationObserver`).
- HTTP Cache, Content Security Policy (`CspPolicy`), WebSockets, BroadcastChannel, Blob, File, ServiceWorker.
- GPU Compositor LayerTree & HarfBuzz Indic Text Shaper (Assamese, Devanagari, BiDi RTL).
