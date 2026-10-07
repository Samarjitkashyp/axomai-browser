# -*- coding: utf-8 -*-
"""Adds the opt-in cleaning hook to crawler/storage.py (run once, on the bot's folder: python3 patch_storage.py <storage.py>)."""
import sys

p = sys.argv[1]
s = open(p, encoding='utf-8').read()
assert 'AXOMAI-CLEAN' not in s, 'already patched'


def rep(old, new):
    global s
    assert old in s, old[:70]
    s = s.replace(old, new, 1)


rep("""        document = {
            "job_id": job_id,""", """        # AXOMAI-CLEAN: sites with a profile (crawler/site_profiles.json) are cleaned; the crawl as it came from the web is archived
        # in crawled_data_raw/ first, and any problem leaves the pages untouched. Other sites are saved exactly as before.
        cleaning = None
        try:
            from crawler.cleaner import apply_profile
            result = apply_profile(seed_url, clean_pages)
            if result is not None:
                if self._archive_raw(job_id, seed_url, status, clean_pages, stats):
                    clean_pages, cleaning = result
                else:
                    cleaning = {"applied": False, "reason": "the raw archive could not be written"}
        except Exception as e:  # never lose a crawl because of the cleaning
            cleaning = {"applied": False, "reason": "error: %s" % e}
        if cleaning is not None:
            stats = dict(stats, cleaning=cleaning)

        document = {
            "job_id": job_id,""")

rep("""    def get_crawl_job(self, job_id: str)""", """    def _archive_raw(self, job_id: str, seed_url: str, status: str, pages: List[Dict[str, Any]], stats: Dict[str, Any]) -> bool:
        \"\"\"Writes the crawl exactly as crawled to crawled_data_raw/ (next to crawled_data). Never overwrites a file.\"\"\"
        try:
            raw_dir = os.path.join(os.path.dirname(self.output_dir), "crawled_data_raw")
            os.makedirs(raw_dir, exist_ok=True)
            path = os.path.join(raw_dir, os.path.basename(self.get_file_path(job_id, seed_url)))
            if os.path.exists(path):
                return True
            document = {
                "job_id": job_id, "seed_url": seed_url, "status": status, "total_pages_crawled": len(pages),
                "created_at": stats.get("started_at", datetime.now().isoformat()),
                "completed_at": stats.get("completed_at", datetime.now().isoformat()),
                "duration_seconds": stats.get("duration_seconds", 0.0), "stats": stats, "pages": pages,
            }
            tmp = path + ".tmp"
            with open(tmp, "w", encoding="utf-8") as f:
                json.dump(document, f, ensure_ascii=False, indent=2)
            os.replace(tmp, path)
            return True
        except Exception:
            return False

    def get_crawl_job(self, job_id: str)""")
open(p, 'w', encoding='utf-8').write(s)
print('storage.py patched')
