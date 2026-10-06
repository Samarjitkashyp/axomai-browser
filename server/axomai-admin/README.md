# Axomai Browser admin panel

A small login-protected page where the owner edits the landing page text: the home page (English and Assamese), the About page, the header menu and the footer links, plus the SEO title and description of each page.
It changes **text and links only**; the design stays as it is in `server/site`.

Address: `https://axomai-browser.aiaxom.co.in/admin-browser-axom/` (not linked from anywhere, `noindex`, listed as `Disallow` in `robots.txt`).

## How it works

* The text of the pages is in `server/site/content.py` (home) and `server/site/about.py` (About). `python3 build_site.py --dump-content` prints all of it as JSON; that is what the editor shows.
* Edits are stored as **only what differs from those defaults** in `/var/www/axomai-browser/data/admin/overrides.json`. New fields added to the code later still show up.
* **Preview** builds the pages into a temporary folder with no Google tags and a "PREVIEW" banner. The live site is not touched.
* **Publish** builds into a staging folder first. Only if the build works, the previous overrides are copied to `data/admin/backups/` (the last 60 are kept), the new ones are saved and the files are copied into `/var/www/axomai-browser/site/`. A failed build changes nothing.
* Version, size and checksum of the installer come from `downloads/release.json` and the `.sha256` file at build time, so a new installer only needs a Publish to refresh the pages.
* Every value is checked against the shape of the default (text stays text, rows keep their number of columns), so a wrong edit cannot break the page generator.

## Security

* One user, one password. The password is never stored: only a salted scrypt hash, in `/etc/axomai-browser/admin.env` (mode 640, group `axomai-browser`).
* Wrong passwords: 5 tries per address, then 15 minutes locked; 20 wrong tries from anywhere locks everyone for 15 minutes.
* Session cookie: signed, `HttpOnly`, `Secure`, `SameSite=Strict`, valid 12 hours, path `/admin-browser-axom/`. Changing the password signs everyone out.
* Every change request needs a custom header and the same origin, which a page on another site cannot send.
* The service listens on `127.0.0.1:8012` only, runs as the `axomai-browser` user, may write only to `site/` and `data/`, and has a 400 MB memory cap.

## Setting up (first time)

On the server (the files go to `/var/www/axomai-browser/`):

```bash
sudo mkdir -p /var/www/axomai-browser/admin /var/www/axomai-browser/site-src
# admin service
sudo cp server.js index.html set-password.js /var/www/axomai-browser/admin/
# the page generator (everything the build needs)
sudo cp build_site.py content.py about.py site.css sitemap.xsl favicon.ico /var/www/axomai-browser/site-src/
sudo cp -r assets /var/www/axomai-browser/site-src/
sudo chown -R axomai-browser:axomai-browser /var/www/axomai-browser/site          # the service publishes here
sudo cp axomai-browser-admin.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now axomai-browser-admin
# add nginx-admin.conf to the server block, then
sudo nginx -t && sudo systemctl reload nginx
```

**Set the password** (typed on the server only, nothing is shown while typing):

```bash
sudo node /var/www/axomai-browser/admin/set-password.js
```

## After changing the page generator or the default text

The editor reads the defaults when the service starts, so after copying new `content.py`, `about.py`, `build_site.py` or `site.css` into `site-src/`:

```bash
sudo systemctl restart axomai-browser-admin
```

Then press **Publish** once in the panel to rebuild the live pages with the new code.

## Files

| File | What it is |
| --- | --- |
| `server.js` | The service (no dependencies): login, content, preview, publish, backups |
| `index.html` | The editor page (one file, no libraries) |
| `set-password.js` | Sets the password on the server |
| `axomai-browser-admin.service` | systemd unit |
| `nginx-admin.conf` | The `location` blocks for Nginx |
