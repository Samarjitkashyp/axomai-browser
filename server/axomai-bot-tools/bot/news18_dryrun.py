# -*- coding: utf-8 -*-
"""Dry run of the assam.news18.com profile on all saved news18 crawls (one version per URL). Writes only /tmp/n18_cleaned.json."""
import collections
import glob
import json
import sys
from urllib.parse import urlparse

sys.path.insert(0, sys.argv[1] if len(sys.argv) > 1 else "/tmp/botcopy3")
from crawler import cleaner  # noqa: E402

BOT = "/home/admin/axomai-bot"
prof = json.load(open(BOT + "/crawler/site_profiles.json" if len(sys.argv) < 3 else sys.argv[2], encoding="utf-8"))["assam.news18.com"]
files = []
for f in glob.glob(BOT + "/crawled_data/*news18*.json"):
    d = json.load(open(f, encoding="utf-8"))
    files.append((d.get("completed_at") or "", d))
pages = {}
for _, d in sorted(files, key=lambda x: x[0]):  # the newest version of a URL wins
    for p in d["pages"]:
        pages[p["url"]] = {k: p.get(k) for k in ("url", "title", "content", "word_count", "change_status")}
pages = list(pages.values())
out, s = cleaner.clean_pages(pages, prof)
print("input (all news18 crawls, one version per URL):", len(pages), "pages")
print("applied:", s["applied"], s.get("reason", ""))
if s["applied"]:
    stay = [p for p in pages if p["url"] not in s["pages_dropped"]]
    wb = sum(len((p["content"] or "").split()) for p in stay)
    print("pages out:", s["pages_out"], "| dropped:", len(s["pages_dropped"]))
    print("words of the pages that stay: %d -> %d (%.0f%% kept)" % (wb, sum(len(p["content"].split()) for p in out), 100.0 * sum(len(p["content"].split()) for p in out) / max(1, wb)))
    print("lines removed from every page they were on (kept in the summary):", len(s["sitewide_lines"]), s["sitewide_lines"][:6])
    print("hosts left:", dict(collections.Counter(urlparse(p["url"]).netloc for p in out)))
    art = [p for p in out if "/news/assam/" in p["url"]][0]
    orig = [p for p in pages if p["url"] == art["url"]][0]
    print("\nexample:", art["url"][-70:], "| words", len(orig["content"].split()), "->", len(art["content"].split()))
    print("BEFORE starts:", (orig["content"] or "")[:300].replace("\n", " | "))
    print("AFTER  starts:", art["content"][:300].replace("\n", " | "))
    worst = sorted(out, key=lambda p: len(p["content"].split()) / max(1, len(next(o for o in pages if o["url"] == p["url"])["content"].split())))[:3]
    print("\nthree pages that lose the most:")
    for p in worst:
        o = next(o for o in pages if o["url"] == p["url"])
        print("  ", p["url"][-60:], len(o["content"].split()), "->", len(p["content"].split()))
    json.dump({"seed_url": "https://assam.news18.com/", "job_id": "job_cleanup_news18", "pages": out}, open("/tmp/n18_cleaned.json", "w", encoding="utf-8"), ensure_ascii=False)
