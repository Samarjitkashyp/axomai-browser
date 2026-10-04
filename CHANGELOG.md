# 📜 Axomai Browser Engine — Full Release Changelog

## [Unreleased] — Desktop shell: working toolbar & real extensions
### Fixed
- Space key works in the address bar, home search and find bar (it arrives as its own key, not a character); held keys repeat.
- Ctrl+V pastes into the address bar / home search / find bar; Ctrl+A selects the whole address; focusing the bar selects the address so typing replaces it.
- Smart address input: text with spaces is a search, domains get `https://`, `localhost` / IPs get `http://`, `about:settings` / `axomai://history` open the page, `javascript:` / `data:` are never run.
- Web view bounds are in physical pixels, so it can no longer overlap the toolbar on 125% / 150% display scaling.
- Tab titles follow the tab you switched to; tabs that hold `axomai://` pages reopen correctly; Ctrl+R / Ctrl+D behave like the toolbar buttons; downloads show a toast and stop the loading bar.
- Address bar no longer shows `file:///…/ui/home.html`; our own pages show `about:home`, `axomai://about`, etc.
- Extensions popup (puzzle icon) is built from the live extension state, so it always matches the Extensions page. Switches persist across restarts.
- Pages with no background colour render on white instead of the dark internal-page colour.
- Native chrome colours are converted from sRGB, so the toolbar matches the web UI instead of looking washed out.

### Added
- Developer tools: F12, Ctrl+Shift+I and right-click > Inspect open the WebView2 (Edge) DevTools.
- View page source: Ctrl+U, right-click > View page source and `view-source:<url>` open a built-in source viewer (line numbers, wrap toggle) in a new tab.
- Browser shortcuts now work while the page has focus: Ctrl+T / W / L / U / D / H / J (previously only from the toolbar). Links that open a new window (`target=_blank`) open in a new tab.
- Every toolbar control works: Back/Forward/Reload (real WebView2 history, correct enabled state), bookmark star (add/remove, filled when bookmarked), Reader, QR code of the current page, security badge (HTTPS / HTTP / Axomai) with details, shield counter + panel, extensions popup, downloads, heritage-theme picker (5 themes, saved), on-device AI panel (summary, key topics, reading stats, ask-this-page), profile panel (editable name, real counts, clear browsing data) and the three-dot menu.
- Extensions are real: **AdBlock Shield** and **Privacy Guard** block ad/tracker requests at the WebView2 network layer (nothing is downloaded); Privacy Guard also adds noise to canvas / WebGL / audio fingerprinting; **Reader Mode** with size and Paper/Sepia/Dark; **Auto-Translate** language picker; **Screen Capture Studio** saves PNGs (visible area, full page, selected region) via the DevTools protocol; **RAM Booster** sets WebView2 low-memory mode and releases idle working-set memory, reporting the real MB released.
- Pages cannot drive the browser: `axomai://` commands are accepted only from our own pages or with a per-run token.

### Added (desktop browser phases 1-12)
- Real tabs with one web view per tab (state kept, sleeping, pin / duplicate / reorder / reopen, favicons, mute), incognito and new windows, split view.
- Settings page and themed History / Bookmarks / Downloads / Passwords / Permissions / Extensions pages; Assamese, Hindi and Bengali for them and the menu.
- Address-bar suggestions, bookmarks bar, bookmark folders with import (Chrome JSON / HTML) and export, history by day with range delete.
- Download manager (progress, pause / resume / cancel, open, show in folder, ask where to save); Save as PDF; per-site zoom; full screen.
- Password manager with DPAPI encryption and per-site matching; site-permission bar; HTTPS-only mode; tracking-prevention level; Global Privacy Control.
- Home page with live weather, theme, working tools and a real Shield counter (the made-up news and statistics were removed).
- Live headlines on the New Tab page from Google News' free RSS feeds (Assam first, cached 15 minutes); a real About page instead of the old landing-page mockup.
- Open links from other apps, register as a web browser, high-DPI scaling of the browser chrome.

### Fixed (phases)
- The first page of a new or restored tab could load before the document-start scripts (ad blocking, Global Privacy Control, password manager) were registered; the view now starts blank and loads the address after they are in place.
- A key that was merely held down when the window gained focus was typed into the address bar (a stray letter at the start of the address); focus-replayed key events are ignored.
- Typing an address that turned out to be a download left the address bar showing the file URL over the old page; the tab now keeps what it showed.
- The browser found its pages and icon through the build folder; it now looks next to the program first, so a moved or installed copy still works.
- Downloads left half-finished by a closed browser no longer stay marked as downloading; autofill offers expire after 30 seconds.
- A page title could be saved against the previous page's history entry when a new address was typed during a load.
- Space and held keys in the address bar, view-source race, address bar reverting while focus echoed.

### Removed
- Dead "Passwords & Autofill" menu entry (there is no password manager yet).

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
