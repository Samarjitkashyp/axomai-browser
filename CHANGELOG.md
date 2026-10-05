# 📜 Axomai Browser Engine — Full Release Changelog

## [`v4.1.1`] — No console window (2026-10-05)
### Fixed in 4.1.1
- The installed browser opened a black console (shell) window next to it, because the release build was a console program. It is now a normal Windows program with no console. What it used to print goes to `%APPDATA%\AxomaiBrowser\axomai.log` (restarted when it passes 512 KB); `cargo run` builds still print to the terminal.

## [`v4.1.0`] — Axom AI chat and our own download server (2026-10-05)
### Added in 4.1
- **Axom AI chat**: the AI button now opens a real chat. Ask anything, or leave "Use this page to answer" on and ask about the page you are reading; answers stream in word by word. Quick starters: summarize, key points, explain simply. New chat clears it; chats are not saved.
- **The OpenAI key and the model are on the server, not in the browser.** The browser talks only to `https://axomai-browser.aiaxom.co.in/v1/chat`; that proxy (`server/axomai-chat`) adds the key, a fixed prompt and a fixed model (`gpt-5.4-mini`), so no user can read the key or pick a model. Daily limits: 30 messages per installation, 60 per network address, 500 for everyone together, at most 6 per minute.
- **Download page and updates from our own server**: `https://axomai-browser.aiaxom.co.in/` has the Windows installer; the browser's update check now reads `/downloads/release.json` there (still checked against the published SHA-256 before anything installs).

### Notes for 4.1
- The chat needs the OpenAI key to be put in `/etc/axomai-browser/chat.env` on the server and a Cloudflare DNS record for `axomai-browser.aiaxom.co.in`; until then the chat says it is not switched on yet. See `server/README.md`.
- Tested with a pretend OpenAI behind the real proxy code (streaming, fixed model, page text, limits, wrong requests, upstream errors). It has not been tried with the real OpenAI key yet.
- What you type, and the page text if you leave that on, goes to OpenAI through the server. The chat box says so.

## [`v4.0.0`] — Desktop browser, fourth generation (2026-10-05)
### Added in 4.0
- **Chrome extensions (Load unpacked)**: Extensions page > Chrome extensions > Load unpacked… copies an unpacked extension folder (one with `manifest.json`, manifest version 2 or 3) into the browser and the web engine loads it. Each one can be switched off or removed; those changes take effect after a restart. Extensions can read the pages you visit, so the page says to add only ones you trust. Tested with a small content-script extension; extensions that depend on Chrome's own services will not work.
- **Installer and updates**: `installer/build-installer.ps1` builds `Axomai-Setup-<version>.exe` (a small C# setup program compiled with the compiler that ships with Windows, with the application embedded) plus a `.sha256` file. It installs for the current user without administrator rights, adds Start menu and desktop shortcuts and an "Installed apps" entry, and `uninstall.ps1` removes it again (your data is kept). The browser checks GitHub releases once a day (switch in Settings), tells you when a newer release exists and, on "Update now", downloads the installer, **checks its SHA-256 against the published `.sha256`**, closes itself, lets the installer replace the files and starts the new version with your tabs back.
- **Sync between your computers, without a server**: Settings > Sync through a folder. Choose a folder that OneDrive, Google Drive or Dropbox shares between your PCs and a passphrase (8+ characters). Axomai keeps one end-to-end encrypted file there (AES-256-GCM, key from the passphrase with PBKDF2-SHA256, 200,000 rounds); the passphrase is never stored. Bookmarks, reading list, notes, site rules, sessions and search shortcuts sync, and saved logins too if you switch that on. It adds and combines; deleting something on one PC does not delete it on the others.
- **Proxy server**: Settings > Network accepts `host:port`, `http://host:port` or `socks5://host:port` for all web pages. It is read when the browser starts, so there is a "Restart browser" button that brings your tabs back. This is a proxy, not a VPN.
- **Windows Hello for passwords** (Settings > Privacy, off by default): before a saved password is copied or exported, Windows asks for your face, fingerprint or PIN. If Windows Hello is not set up the action is refused with an explanation.

### Fixed in 4.0
- Release builds failed to compile because the DevTools button used a function that only exists in debug builds; it now asks the web engine directly.

### Not done in 4.0 (and why)
- **A real VPN** needs servers and a service behind it; code in the browser cannot provide that. The proxy setting is the honest part of it.
- **AI chat sidebar**: left out on purpose, since you asked for the assistant to be removed; it would also need an API key or a service.
- **Android app**: a separate project.
- **Passkeys**: web sites' passkey (WebAuthn) sign-in is handled by the web engine and Windows; it was not tested here because this PC has no Windows Hello hardware.

### Notes for 4.0
- Tested on this PC: loading an extension and seeing it run, switching it off and restarting, the proxy through a local test proxy, Windows Hello refusal when it is not set up, installing, running and uninstalling the program, the whole update flow against a local test server (including refusing a wrong checksum), and sync between two separate profiles (including refusing a wrong passphrase). Not tested: the positive Windows Hello prompt (no Hello hardware) and the folder pickers' dialogs.
- The installer is not code-signed, so Windows SmartScreen may warn when it is first run.
- The engine crate (`native/rust_engine`) is unchanged and keeps its own 1.6.0 version.

## [`v3.0.0`] — Desktop browser, third generation (2026-10-05)
### Added in 3.0
- **Picture-in-picture**: menu > Picture in picture floats the playing video in a small window; choose it again to leave.
- **Translate page**: menu > Translate page… (also the Auto-Translate extension) opens the page through Google Translate in Assamese, Hindi, Bengali and nine other languages.
- **Read aloud**: menu > Read aloud, and a Listen button with a speed switch in Reader Mode, read the page (or your selection) with the Windows voices.
- **Sessions**: menu > Save tabs as a session… names the open tabs; the Sessions page opens a whole set again.
- **Password import / export**: Chrome-compatible CSV on the Passwords page. Export is plain text and the page warns about it first.
- **Backup and restore** (Settings > Your data): bookmarks, history, settings, reading list, notes, site rules and sessions in one JSON file; restoring only adds, it never deletes. Passwords are not in the backup.
- **Voice commands**: menu > Voice commands listens with the Windows speech recogniser for about 45 phrases ("new tab", "go back", "zoom in", "scroll down", "read aloud", and a few Hindi / Assamese phrases written in English letters).
- **Screenshot editor**: after a capture, the Edit button on the toast opens pen, arrow, box, highlight, blur (pixelate) and text tools; Save PNG downloads the result.
- **Site settings**: menu > Site settings gathers zoom, always-mute, block, saved permissions, saved logins and "clear cookies and site data" for the open site.
- **Developer panel**: menu > Developer panel switches this tab's user agent (iPhone, Android, iPad, Firefox, Safari) and mobile / tablet view, and opens DevTools or the page source.
- **Search shortcuts**: `@yt`, `@wiki`, `@as`, `@gh`, `@maps`, `@news`, `@amazon`, `@g`, `@bing`, `@ddg` in the address bar and the New Tab search box, plus your own under Settings > Search shortcuts.

### Fixed in 3.0
- The New Tab search box now follows the address-bar rules (`localhost:8765/page` used to become a web search).

### Notes for 3.0
- Read aloud uses the voices Windows has installed; this PC has English voices only, so other languages are refused with a message instead of being read wrongly. If Windows has no working audio output the browser says so.
- Voice commands need a microphone. They were tested with synthesised speech fed to the recogniser, not with a live microphone. The recogniser is English (US); the Hindi / Assamese phrases match by sound.
- Translate depends on Google Translate's page proxy; picture-in-picture depends on the page's video.
- The backup file picker and the CSV file picker were not driven in a live run (their file handling is unit-tested).
- Site rules (mute, block) and the site-settings toggles need an address with a dot (`example.com`), not `localhost`.
- The engine crate (`native/rust_engine`) is unchanged and keeps its own 1.6.0 version.

## [`v2.0.0`] — Desktop browser, second generation (2026-10-04)
### Added in 2.0
- **Tab search** (Ctrl+Shift+A): a popup that filters every open tab by title or address; Enter switches to it.
- **Tab groups**: give tabs a name and one of six colours (tab menu > Add to new group); grouped tabs sit together, show a coloured bar and are restored with the session.
- **Reading list**: save a page to read later (menu > Add to reading list), mark items read / unread, clear the read ones; its own page with site icons.
- **Notes on pages**: sticky notes (menu > Add note to this page) that return every time you open the same page, are edited in place and are listed on the Notes page. Not saved in private windows.
- **Dark mode for websites** (Settings, off by default): inverts light pages and leaves images, video and already-dark pages alone.
- **Hide cookie banners** (on by default): hides the common cookie-consent pop-ups and unlocks scrolling they locked. It hides them, it does not click "reject".
- **Skip YouTube ads** (on by default): clicks Skip, fast-forwards unskippable ads, hides ad slots and switches next-video autoplay off.
- **Clean printouts** (on by default): menus, ads, comments, sidebars and iframes are left out when you print or save as PDF.
- **Better find in page**: every match is highlighted, the current one in orange, with an "n / total" count.
- **Mute site and Block site**: a muted site stays silent in every tab; a blocked site shows a block page with an Unblock button. Both are managed under Settings > Muted and blocked sites.
- **35 weather cities** for the home page (all main Assam towns, the other north-eastern capitals and the big Indian cities) instead of 7.
- Settings keep their scroll position when a site rule is added or removed.

### Fixed in 2.0
- Document-start page scripts could throw before the page had a root element and silently stop; they now wait for it, and each tweak runs on its own so one failing cannot stop the others.

### Notes for 2.0
- The YouTube and cookie-banner scripts depend on those sites' current markup and may need updating when it changes; they were tested against local test pages, not against the live sites.
- The engine crate (`native/rust_engine`) is unchanged and keeps its own 1.6.0 version.

## [`v2.0.0` earlier work] — Desktop shell: working toolbar & real extensions
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
- Live headlines on the New Tab page from Assam newspapers' and Google News' free RSS feeds (Assam first, with photos or the publisher's logo, cached 15 minutes); a real About page instead of the old landing-page mockup.
- Site icons on the bookmarks bar, the Bookmarks page and the History page (saved from the pages you visit, fetched once for bookmarks that have none).
- Open links from other apps, register as a web browser, high-DPI scaling of the browser chrome.

### Fixed (phases)
- Opening the browser with bookmarks whose icons were not saved yet could crash at startup; icon fetching now waits until the first tab exists.
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
