# Axomai Browser

**A free web browser made in Assam, with a built-in AI assistant. For Windows and Android.**

![Rust](https://img.shields.io/badge/Rust-1.88-000000?logo=rust&logoColor=white)
![WebView2](https://img.shields.io/badge/Engine-WebView2%20(Chromium)-0078D4?logo=microsoftedge&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-v4.1.2-0078D6?logo=windows&logoColor=white)
![Android](https://img.shields.io/badge/Android-v1.0.0-3DDC84?logo=android&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green)

| | |
|---|---|
| **Download** | https://axomai-browser.aiaxom.co.in/ |
| **Latest release** | Windows v4.1.2 (Windows 10 / 11, 64-bit) · Android v1.0.0 (Android 7.0 or newer, APK) |
| **Docs** | [Changelog](CHANGELOG.md) · [Server notes](server/README.md) · [Architecture](ARCHITECTURE.md) · [Reality audit](REALITY_AUDIT.md) |

Axomai Browser is a product of [Axom AI](https://aiaxom.co.in/), an artificial intelligence company from Assam, India.

---

## Overview

The desktop browser is written in Rust, and there is an Android app built with Flutter. Web pages are shown by the Microsoft Edge WebView2 (Chromium) engine, so sites work the way they do in Chrome or Edge. The tab strip, toolbar and built-in pages are Axomai's own.

## Features

| Area | What you get |
| --- | --- |
| **Axom AI chat** | Ask anything, or ask about the page you are reading. Answers stream in. Chats are not saved. |
| **Tabs and windows** | Real tabs, pin / duplicate / reorder / reopen, tab groups and search, sleeping tabs, split view, incognito, extra windows. |
| **Address bar** | Suggestions from bookmarks and history, bookmarks bar, search-engine choice, per-site zoom, full screen, print / save as PDF. |
| **Library** | History with search, bookmark folders (Chrome / HTML import, HTML export), download manager with pause and resume, reading list, page notes. |
| **Privacy** | Ad and tracker blocking, fingerprint noise, HTTPS-only mode, Global Privacy Control, per-site permissions, cookie-banner hiding, incognito. Optional Windows Hello before saved passwords are copied. |
| **New Tab** | Headlines from Assamese and North-East news sources with photos, weather for 35 towns, quick tools (QR, image compress, translator). |
| **Languages and themes** | Menus and settings in অসমীয়া, हिन्दी, বাংলা and English. Five heritage themes: Tea Garden, Kaziranga Mist, Brahmaputra Azure, Gamosa Crimson, Obsidian Dark Glass. |
| **Page tools** | Translate, read aloud, picture-in-picture, screenshot editor, dark mode for websites, voice commands, clean printouts. |
| **Extensions** | Chrome extensions via *Load unpacked*. |
| **Sync** | End-to-end encrypted sync of bookmarks and settings through a folder you share with OneDrive, Google Drive or Dropbox. No server involved. |
| **Updates** | One-click updates on Windows, checked against a published SHA-256 before anything installs. |
| **Android app** | Tabs, bookmarks, history, Axom AI chat, the Assam New Tab page and the same five themes on your phone. Downloaded as an APK from the website. |

## Install

### Windows

1. Download `Axomai-Setup-<version>.exe` from https://axomai-browser.aiaxom.co.in/.
2. Run it. It installs for the current user, with no administrator rights needed, and adds Start menu and desktop shortcuts.
3. Windows may show a blue "Windows protected your PC" screen, because the installer is not code-signed yet: choose **More info**, then **Run anyway**.

Remove it from *Installed apps*, or run `uninstall.ps1` from the install folder.

### Android

1. On your phone, open https://axomai-browser.aiaxom.co.in/ and tap **Download for Android** (`Axomai-Browser-<version>.apk`, about 55 MB).
2. Open the file. If Android asks, allow installs from your browser or Files app for this one time.
3. The app is not on Google Play yet, so Android may show a Play Protect or "unknown app" message. If you downloaded it from the website, choose **Install anyway**.

Needs Android 7.0 or newer. You can compare the SHA-256 on the download page with the file you got. There is no iPhone version yet.

## Architecture

```
  Windows app (Rust, tao + wry)           Android app (Flutter, separate project)
  ┌───────────────────────────────┐        ┌──────────────────────────┐
  │ Tabs, toolbar, built-in pages │        │ Tabs, bookmarks, chat    │
  │ (ui/) · settings · sync       │        └────────────┬─────────────┘
  │ updater · privacy shield      │                     │
  │ Web pages: Edge WebView2      │                     │
  └──────────────┬────────────────┘                     │
                 │  HTTPS                               │
                 ▼                                      ▼
        axomai-browser.aiaxom.co.in  (nginx + Cloudflare)
        ├─ /v1/chat      chat proxy (Node)    adds the key, prompt and model, enforces limits
        ├─ /downloads/   release.json · android.json · installers · SHA-256
        └─ /             landing page, generated by server/site
```

Web pages are drawn by WebView2, so they behave as in Chrome or Edge. Everything around the page (tab strip, address bar, New Tab, settings, sync, updates, privacy features) is Axomai's own Rust and web UI code. `native/rust_engine/` is a separate, experimental from-scratch engine and is not part of the shipped browser; see [REALITY_AUDIT.md](REALITY_AUDIT.md).

## How the AI chat works

Neither the desktop browser nor the Android app holds an OpenAI key. They talk to `https://axomai-browser.aiaxom.co.in/v1/chat`, a small proxy on our server that adds the key, a fixed prompt and a fixed model, and applies daily limits. What you type, and the page text if "Use this page" is on, is sent to OpenAI through that server. The server keeps only daily counters; chat text is never stored. Details are in [server/README.md](server/README.md).

## Build from source

You need Windows, [Rust](https://rustup.rs/) (the repo pins 1.88 in `rust-toolchain.toml`) and the Visual Studio C++ build tools.

```bash
cd rust_desktop
cargo run              # debug build, prints logs to the terminal
cargo test             # automated tests
cargo build --release  # release build, no console window
```

A release build writes its log to `%APPDATA%\AxomaiBrowser\axomai.log`.

Build the installer and the update file (needs only Windows itself):

```bash
powershell -File installer\build-installer.ps1 -Exe <path to axomai_browser.exe> -Out <folder>
powershell -File installer\make-release-json.ps1 -Dir <folder> -Version <x.y.z> -Notes "What's new"
```

Tests for the chat proxy: `node server/axomai-chat/test.js` (runs against a pretend OpenAI, no key needed).

## Releases

1. Build the installer and the update file with the two PowerShell scripts above.
2. Upload the installer, its `.sha256` and `release.json` to `/downloads/` on the server. The browser's update check reads `release.json` and installs only if the published SHA-256 matches.
3. Android: publish the APK, its `.sha256` and `android.json` the same way.

## Website and admin panel

The website https://axomai-browser.aiaxom.co.in/ (landing page in English and Assamese, About page, downloads, sitemap) is generated by `server/site`. The owner edits its text, header menu and footer links in a password-protected admin panel (`server/axomai-admin`); see [server/README.md](server/README.md).

The installers live in `/downloads/` on the server: `release.json` describes the Windows installer (the desktop update check reads it) and `android.json` describes the Android APK (version, file, size, SHA-256).

## Repository layout

| Folder | Contents |
| --- | --- |
| `rust_desktop/` | The desktop browser (window, tabs, settings, sync, updater). |
| `ui/` | Built-in pages: New Tab, About, Settings, sidebar pages. |
| `installer/` | Setup program, uninstaller and release-file scripts. |
| `server/` | Chat proxy (`axomai-chat`), the landing page generator (`site`) and the admin panel (`axomai-admin`). |
| `native/rust_engine/` | An experimental from-scratch rendering engine, kept separate from the desktop browser. See [REALITY_AUDIT.md](REALITY_AUDIT.md) for what is and is not production-grade. |

## Privacy and security

- **No key in the app.** The AI key lives only on the server; the app cannot choose the model or the prompt.
- **Chats are not saved** by the browser or the server (only daily limit counters).
- **Blocking:** ads and trackers, fingerprint noise, HTTPS-only mode, Global Privacy Control, cookie-banner hiding and per-site permissions.
- **Sync** of bookmarks and settings is end-to-end encrypted and goes through a folder you share yourself; no Axomai server is involved.
- **Updates** are checked against a published SHA-256 before anything is installed.
- To report a security problem, email the maintainer directly instead of opening a public issue.

## Limits

- The desktop browser is Windows only. There is no macOS, Linux or iPhone version yet. The Android app is a separate project and is not part of this repository.
- Not a VPN. The proxy setting sends pages through a proxy you provide.
- Chrome Web Store extensions cannot be installed directly; only unpacked extensions load.
- The Windows installer is not code-signed yet, and the Android APK is not on Google Play yet, so both can show a warning on first run.

## Contributing

Issues and pull requests are welcome.

1. Branch from `main`.
2. Keep the change focused; run `cargo test` in `rust_desktop` before sending it.
3. Describe what changed and why in the pull request.

## Licence

MIT © 2026 Samarjit Kashyap. Built with pride in Assam, India.

Maintainer: Samarjit Kashyap, samarjitkashyp@gmail.com
