import sys
import tkinter as tk
from tkinter import ttk, messagebox

from browser.network import URL
from browser.html_parser import HTMLParser, Element, Text
from browser.css_parser import CSSParser, DEFAULT_UA_STYLES, style_tree
from browser.layout import build_layout_tree
from browser.painter import build_display_list
from browser.js_engine import JSEngine

DEFAULT_PAGE = """
<html>
  <head>
    <style>
      body { font-family: sans-serif; background-color: #f4f6f9; }
      .header { background-color: #1e293b; color: #ffffff; padding: 15px; }
      .container { margin: 20px; background-color: #ffffff; padding: 20px; border-radius: 8px; }
      h1 { color: #38bdf8; font-size: 28px; }
      p { color: #334155; font-size: 16px; margin-top: 10px; }
      b { color: #0284c7; }
    </style>
  </head>
  <body>
    <div class="header">
      <h1>🚀 Axomai Browser</h1>
    </div>
    <div class="container">
      <h2>Welcome to Axomai Browser!</h2>
      <p>This browser is built <b>100% from scratch</b> in Python!</p>
      <p>Features supported in this milestone:</p>
      <p>1. 🌐 <b>Network Layer:</b> HTTP, HTTPS, File, and Data URLs socket engine.</p>
      <p>2. 🏗️ <b>HTML Parser:</b> Stack-based DOM Tree builder.</p>
      <p>3. 🎨 <b>CSS Engine:</b> Selectors, Cascade, Inheritance & UA Stylesheets.</p>
      <p>4. 📏 <b>Layout Engine:</b> Box geometry, Block stacking & Inline text wrapping.</p>
      <p>5. 🖌️ <b>Painter:</b> Display command list painted on Tkinter Canvas.</p>
      <p>6. 🔗 <b>Interactive Links:</b> Clickable hyper-linking & relative URL resolution.</p>
      <script>
        var statusMsg = "7. ⚡ JavaScript Engine: Inline script execution & DOM document.write mutation!";
        document.write("<p><b>" + statusMsg + "</b></p>");
      </script>
      <p>Try live websites:</p>
      <p>👉 <a href="https://example.org">Visit Example.org</a></p>
      <p>👉 <a href="https://wikipedia.org">Visit Wikipedia</a></p>
    </div>
  </body>
</html>
"""

WIDTH, HEIGHT = 900, 650
SCROLL_STEP = 40

class AxomaiBrowserGUI:
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.title("Axomai Browser")
        self.root.geometry(f"{WIDTH}x{HEIGHT}")

        self.history = []
        self.history_idx = -1
        self.scroll_y = 0.0
        self.max_scroll_y = 0.0
        self.display_list = []

        self.current_url_obj = None
        self._setup_ui()
        self._bind_events()

        # Load default page on start
        self.load_url("data:text/html," + DEFAULT_PAGE)

    def _setup_ui(self):
        # Top toolbar
        toolbar = ttk.Frame(self.root, padding=5)
        toolbar.pack(fill="x", side="top")

        self.back_btn = ttk.Button(toolbar, text="◀", width=3, command=self.go_back)
        self.back_btn.pack(side="left", padx=2)

        self.forward_btn = ttk.Button(toolbar, text="▶", width=3, command=self.go_forward)
        self.forward_btn.pack(side="left", padx=2)

        self.url_entry = ttk.Entry(toolbar)
        self.url_entry.pack(side="left", fill="x", expand=True, padx=5)
        self.url_entry.bind("<Return>", lambda e: self.navigate())

        self.go_btn = ttk.Button(toolbar, text="Go", width=5, command=self.navigate)
        self.go_btn.pack(side="left", padx=2)

        # Status Bar
        self.status_var = tk.StringVar(value="Ready")
        status_bar = ttk.Label(self.root, textvariable=self.status_var, relief="sunken", anchor="w")
        status_bar.pack(fill="x", side="bottom")

        # Main Rendering Canvas
        self.canvas = tk.Canvas(self.root, bg="white")
        self.canvas.pack(fill="both", expand=True, side="top")

    def _bind_events(self):
        self.canvas.bind("<Configure>", lambda e: self.render())
        self.canvas.bind("<Button-1>", self._on_canvas_click)
        self.canvas.bind("<Motion>", self._on_canvas_motion)
        self.root.bind("<Up>", lambda e: self.scroll(-SCROLL_STEP))
        self.root.bind("<Down>", lambda e: self.scroll(SCROLL_STEP))
        self.root.bind("<MouseWheel>", lambda e: self.scroll(-int(e.delta / 2)))

    def _on_canvas_click(self, event):
        click_x = event.x
        click_y = event.y + self.scroll_y
        for cmd in self.display_list:
            if hasattr(cmd, "href") and cmd.href:
                if cmd.x <= click_x <= cmd.x + cmd.width and cmd.y <= click_y <= cmd.y + cmd.height:
                    target = self.current_url_obj.resolve(cmd.href) if self.current_url_obj else cmd.href
                    self.load_url(target)
                    break

    def _on_canvas_motion(self, event):
        hover_x = event.x
        hover_y = event.y + self.scroll_y
        hovered_href = None
        for cmd in self.display_list:
            if hasattr(cmd, "href") and cmd.href:
                if cmd.x <= hover_x <= cmd.x + cmd.width and cmd.y <= hover_y <= cmd.y + cmd.height:
                    hovered_href = cmd.href
                    break

        if hovered_href:
            self.canvas.config(cursor="hand2")
            target = self.current_url_obj.resolve(hovered_href) if self.current_url_obj else hovered_href
            self.status_var.set(f"Link: {target}")
        else:
            self.canvas.config(cursor="")

    def navigate(self):
        url_str = self.url_entry.get().strip()
        if not url_str:
            return
        if not (url_str.startswith("http://") or url_str.startswith("https://") or 
                url_str.startswith("file://") or url_str.startswith("data:")):
            url_str = "https://" + url_str

        self.load_url(url_str)

    def _short_url(self, url_str: str) -> str:
        clean = url_str.split("\n", 1)[0]
        if len(clean) > 50:
            return clean[:47] + "..."
        return clean

    def load_url(self, url_str: str):
        short = self._short_url(url_str)
        self.status_var.set(f"Connecting to {short}...")
        self.root.update_idletasks()

        try:
            url = URL(url_str)
            self.current_url_obj = url
            headers, body = url.request()

            # Record history
            if not self.history or self.history[self.history_idx] != url_str:
                self.history = self.history[:self.history_idx + 1]
                self.history.append(url_str)
                self.history_idx = len(self.history) - 1

            self.url_entry.delete(0, tk.END)
            self.url_entry.insert(0, url_str)

            # Pipeline execution
            self.status_var.set("Parsing HTML & CSS...")
            dom_root = HTMLParser(body).parse()

            # Execute inline JavaScript <script> tags
            scripts = []
            self._extract_script_tags(dom_root, scripts)
            if scripts:
                self.status_var.set("Executing JavaScript...")
                js_engine = JSEngine(dom_root)
                for js_code in scripts:
                    js_engine.execute(js_code)

            author_css = ""
            # Extract inline <style> contents
            self._extract_style_tags(dom_root, author_css_list:=[])
            author_css = "\n".join(author_css_list)

            ua_rules = CSSParser(DEFAULT_UA_STYLES).parse()
            author_rules = CSSParser(author_css).parse()
            all_rules = ua_rules + author_rules

            style_tree(dom_root, all_rules)

            self.status_var.set("Computing Layout...")
            layout_root = build_layout_tree(dom_root)

            canvas_width = max(self.canvas.winfo_width(), 800)
            if layout_root:
                total_height = layout_root.layout(0, 0, canvas_width)
                self.max_scroll_y = max(0, total_height - self.canvas.winfo_height())
                self.display_list = build_display_list(layout_root)
            else:
                self.display_list = []

            self.scroll_y = 0.0
            self.render()
            self.status_var.set(f"Loaded {short}")
            self._update_nav_buttons()

        except Exception as e:
            messagebox.showerror("Page Load Error", f"Failed to load {short}:\n{e}")
            self.status_var.set(f"Error loading {short}")

    def _extract_style_tags(self, node, css_list: list):
        if isinstance(node, Element) and node.tag == "style":
            for child in node.children:
                if isinstance(child, Text):
                    css_list.append(child.text)
        for child in getattr(node, "children", []):
            self._extract_style_tags(child, css_list)

    def _extract_script_tags(self, node, script_list: list):
        if isinstance(node, Element) and node.tag == "script":
            for child in node.children:
                if isinstance(child, Text):
                    script_list.append(child.text)
        for child in getattr(node, "children", []):
            self._extract_script_tags(child, script_list)

    def render(self):
        self.canvas.delete("all")
        for cmd in self.display_list:
            cmd.execute(self.canvas, self.scroll_y)

    def scroll(self, delta: float):
        self.scroll_y = max(0.0, min(self.scroll_y + delta, self.max_scroll_y))
        self.render()

    def go_back(self):
        if self.history_idx > 0:
            self.history_idx -= 1
            self.load_url(self.history[self.history_idx])

    def go_forward(self):
        if self.history_idx < len(self.history) - 1:
            self.history_idx += 1
            self.load_url(self.history[self.history_idx])

    def _update_nav_buttons(self):
        self.back_btn["state"] = "normal" if self.history_idx > 0 else "disabled"
        self.forward_btn["state"] = "normal" if self.history_idx < len(self.history) - 1 else "disabled"


def main():
    root = tk.Tk()
    app = AxomaiBrowserGUI(root)
    if len(sys.argv) > 1:
        app.load_url(sys.argv[1])
    root.mainloop()


if __name__ == "__main__":
    main()
