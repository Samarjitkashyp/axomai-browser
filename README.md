# Axomai Browser

**A free Windows web browser made in Assam, with a built-in AI assistant.**

[Download](https://axomai-browser.aiaxom.co.in/) · [Changelog](CHANGELOG.md) · [Server notes](server/README.md) · [Architecture](ARCHITECTURE.md) · [Reality audit](REALITY_AUDIT.md)

Latest release: **v4.1.2** · Windows 10 / 11 (64-bit) · MIT licence

---

## What it is

Axomai Browser is a desktop browser written in Rust. Web pages are shown by the Microsoft Edge WebView2 (Chromium) engine, so sites work the way they do in Chrome or Edge. The tab strip, toolbar and built-in pages are Axomai's own.

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
| **Updates** | One-click updates, checked against a published SHA-256 before anything installs. |

## Install

1. Download `Axomai-Setup-<version>.exe` from https://axomai-browser.aiaxom.co.in/.
2. Run it. It installs for the current user, with no administrator rights needed, and adds Start menu and desktop shortcuts.
3. Windows may show a blue "Windows protected your PC" screen, because the installer is not code-signed yet: choose **More info**, then **Run anyway**.

Remove it from *Installed apps*, or run `uninstall.ps1` from the install folder.

## How the AI chat works

The browser never holds an OpenAI key. It talks to `https://axomai-browser.aiaxom.co.in/v1/chat`, a small proxy on our server that adds the key, a fixed prompt and a fixed model, and applies daily limits. What you type, and the page text if "Use this page" is on, is sent to OpenAI through that server. Details are in [server/README.md](server/README.md).

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

## Repository layout

| Folder | Contents |
| --- | --- |
| `rust_desktop/` | The desktop browser (window, tabs, settings, sync, updater). |
| `ui/` | Built-in pages: New Tab, About, Settings, sidebar pages. |
| `installer/` | Setup program, uninstaller and release-file scripts. |
| `server/` | Chat proxy and the landing page generator (`server/site`). |
| `native/rust_engine/` | An experimental from-scratch rendering engine, kept separate from the desktop browser. See [REALITY_AUDIT.md](REALITY_AUDIT.md) for what is and is not production-grade. |

## Limits

- Windows only. There is no Android, macOS or Linux build of the desktop browser.
- Not a VPN. The proxy setting sends pages through a proxy you provide.
- Chrome Web Store extensions cannot be installed directly; only unpacked extensions load.
- The installer is not code-signed yet.

## Contributing

Issues and pull requests are welcome. Run `cargo test` in `rust_desktop` before sending a change.

## Licence

MIT © 2026 Samarjit Kashyap. Built with pride in Assam, India.
