# Axomai Browser server (axomai-browser.aiaxom.co.in)

Everything the browser needs from a server lives under `/var/www/axomai-browser/` on the Lightsail machine.

| Folder | What it holds |
| --- | --- |
| `downloads/` | `Axomai-Setup-<version>.exe`, its `.sha256`, and `release.json` (what the browser's update check reads) |
| `site/` | the download page (`server/axomai-chat/site/index.html`) |
| `chat-proxy/` | the chat proxy (`server/axomai-chat/server.js`), a Node service on `127.0.0.1:8011` |
| `data/` | daily-limit counters only (`usage.json`); chat text is never stored |

Outside that folder, where Linux expects them:

* `/etc/axomai-browser/chat.env`: **the OpenAI key and the model.** Owner root, group `axomai-browser`, mode 640. Typed on the server by the owner; never in git.
* `/etc/nginx/conf.d/axomai-browser.aiaxom.co.in.conf`: Nginx (same pattern as the other aiaxom.co.in sites: Cloudflare Origin CA wildcard certificate, Cloudflare proxy in front).
* `/etc/systemd/system/axomai-browser-chat.service`: the proxy service, run as the `axomai-browser` user with a 300 MB memory cap.

## How the chat works

The browser sends the conversation (and the page text, if "Use this page" is on) to `https://axomai-browser.aiaxom.co.in/v1/chat`.
The proxy adds the key, a fixed system prompt and the fixed model (`OPENAI_MODEL`), calls OpenAI and streams the answer back.
The browser cannot choose the model, the key or the prompt. Limits (UTC day): `LIMIT_PER_CLIENT` (30), `LIMIT_PER_IP` (60),
`LIMIT_GLOBAL` (500, for everyone together) and `LIMIT_BURST_PER_MINUTE` (6).

## First-time setup / changes

1. Cloudflare DNS: `A  axomai-browser  3.6.237.64`, proxied (orange cloud).
2. On the server: `sudo nano /etc/axomai-browser/chat.env`, put the key after `OPENAI_API_KEY=`, then `sudo systemctl restart axomai-browser-chat`.
3. Check: `curl http://127.0.0.1:8011/health` shows `"configured":true`.
4. A new browser version: build with `installer/build-installer.ps1`, run `installer/make-release-json.ps1`, upload the three files to `downloads/`.

`node server/axomai-chat/test.js` runs the proxy's tests against a pretend OpenAI; `node server/axomai-chat/mock-openai.js` is that pretend OpenAI for trying the browser without a key
(start the proxy with `OPENAI_API_KEY=x OPENAI_BASE=http://127.0.0.1:9100` and the browser with `AXOMAI_CHAT_URL=http://127.0.0.1:8011/v1/chat`).

## The landing page (`site/`)

`server/site/` builds the page at https://axomai-browser.aiaxom.co.in/ (and `/as/`, `robots.txt`, `sitemap.xml`, `llms.txt`, `llms-full.txt`, the web manifest and icons):

```
python server/site/make_assets.py                 # only when the artwork changes (needs Pillow)
python server/site/build_site.py --version 4.1.1 --size 28 --sha <sha256 of the installer>
scp -r server/site/dist/* admin@<server>:/tmp/axsite/   # then copy into /var/www/axomai-browser/site/
```

Words live in `server/site/content.py`; colours and fonts are the browser's own (Plus Jakarta Sans, the Assam Tea Garden palette, dark mode follows the system).
The page has one H1, answer-first text and a fact table (for answer engines), an FAQ with FAQPage / HowTo / SoftwareApplication / Organization structured data (JSON-LD), canonical and hreflang links,
Open Graph / Twitter cards, `llms.txt` for AI assistants, and a robots.txt that welcomes search and AI crawlers.
The Assamese page is written but marked `noindex` (see `REVIEWED` in `content.py`) until a native speaker has read it; set `'as': True` and rebuild to publish it.
After a new release, rebuild with the new version and checksum so the static text matches (the download button and checksum also refresh themselves from `release.json`).
