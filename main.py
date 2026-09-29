import sys
import tkinter as tk
from tkinter import ttk, messagebox

from browser.network import URL
from browser.html_parser import HTMLParser, Element, Text
from browser.css_parser import CSSParser, DEFAULT_UA_STYLES, style_tree
from browser.layout import build_layout_tree
from browser.painter import build_display_list, DrawInput, DrawButton
from browser.js_engine import JSEngine

DEFAULT_PAGE = """
<html>
  <head>
    <style>
      body { font-family: 'Segoe UI', system-ui, sans-serif; background-color: #ffffff; margin: 0px; color: #202124; }
      .hero { text-align: center; margin-top: 40px; margin-bottom: 20px; }
      .title { font-size: 36px; font-weight: bold; color: #1a73e8; margin-bottom: 10px; }
      .subtitle { font-size: 14px; color: #5f6368; }
      .search-box { margin-top: 25px; margin-bottom: 30px; text-align: center; }
      .container { max-width: 800px; margin-left: auto; margin-right: auto; padding: 20px; }
      .card { background-color: #f8f9fa; border: 1px solid #dadce0; border-radius: 12px; padding: 20px; margin-bottom: 20px; }
      h2 { color: #1a73e8; font-size: 20px; margin-top: 0px; }
      p { color: #3c4043; font-size: 14px; margin-top: 8px; line-height: 1.5; }
      b { color: #1a73e8; }
      a { color: #1a73e8; text-decoration: none; font-weight: bold; }
    </style>
  </head>
  <body>
    <div class="hero">
      <div class="title">🚀 Axomai Chrome Engine</div>
      <div class="subtitle">Built 100% from scratch in Python • High Performance Browser Pipeline</div>
    </div>
    <div class="container">
      <div class="search-box">
        <p><b>Search the Web or Enter URL:</b></p>
        <p>
          <input placeholder="Search Google or type a URL..." value="" width="480px" height="42px" />
          <button width="90px" height="42px">Search</button>
        </p>
      </div>
      <div class="card">
        <h2>⚡ Browser Subsystems Status</h2>
        <p>1. 🌐 <b>Network Sockets:</b> HTTP, HTTPS (SSL), File & Data URLs.</p>
        <p>2. 🏗️ <b>HTML DOM Parser:</b> Stack-based AST & Error Recovery.</p>
        <p>3. 🎨 <b>CSS Style Engine:</b> Cascading rules, selectors & UA styles.</p>
        <p>4. 📏 <b>Layout Engine:</b> Box Model, Text wrapping & Pill geometry.</p>
        <p>5. 🖌️ <b>Painter Engine:</b> Tkinter Display List commands & Canvas drawing.</p>
        <p>6. 🔗 <b>Hyperlinks:</b> Interactive link resolution & hover cursor.</p>
        <script>
          var statusMsg = "7. ⚡ JavaScript Engine: Inline script execution & DOM document.write mutation!";
          document.write("<p><b>" + statusMsg + "</b></p>");
        </script>
        <p>8. 🖼️ <b>Image Subsystem:</b> Pillow PhotoImage rendering & base64 decoding.</p>
        <p>9. 📝 <b>Form Controls:</b> Chrome Pill Input Fields & Canvas Typing.</p>
      </div>
      <div class="card">
        <h2>🌐 Quick Shortcuts</h2>
        <p>👉 <a href="https://example.org">Example.org</a> • <a href="https://wikipedia.org">Wikipedia</a> • <a href="https://github.com/Samarjitkashyp/axomai-browser">Axomai GitHub Repo</a></p>
      </div>
    </div>
  </body>
</html>
"""

WIDTH, HEIGHT = 950, 680
SCROLL_STEP = 40

class AxomaiBrowserGUI:
    def __init__(self, root: tk.Tk):
        self.root = root
        self.root.title("Axomai Browser - Chrome Engine")
        self.root.geometry(f"{WIDTH}x{HEIGHT}")

        self.history = []
        self.history_idx = -1
        self.scroll_y = 0.0
        self.max_scroll_y = 0.0
        self.display_list = []
        self.focused_input = None

        self.current_url_obj = None
        self._setup_ui()
        self._bind_events()

        # Load default page on start
        self.load_url("data:text/html," + DEFAULT_PAGE)

    def _setup_ui(self):
        # Chrome Top Toolbar
        toolbar = ttk.Frame(self.root, padding=6)
        toolbar.pack(fill="x", side="top")

        self.back_btn = ttk.Button(toolbar, text="◄", width=3, command=self.go_back)
        self.back_btn.pack(side="left", padx=2)

        self.forward_btn = ttk.Button(toolbar, text="►", width=3, command=self.go_forward)
        self.forward_btn.pack(side="left", padx=2)

        self.reload_btn = ttk.Button(toolbar, text="↻", width=3, command=self.reload_page)
        self.reload_btn.pack(side="left", padx=2)

        self.home_btn = ttk.Button(toolbar, text="🏠", width=3, command=lambda: self.load_url("data:text/html," + DEFAULT_PAGE))
        self.home_btn.pack(side="left", padx=2)

        self.url_entry = ttk.Entry(toolbar)
        self.url_entry.pack(side="left", fill="x", expand=True, padx=6)
        self.url_entry.bind("<Return>", lambda e: self.navigate())

        self.go_btn = ttk.Button(toolbar, text="Go", width=5, command=self.navigate)
        self.go_btn.pack(side="left", padx=2)

        # Status Bar
        self.status_var = tk.StringVar(value="Ready")
        status_bar = ttk.Label(self.root, textvariable=self.status_var, relief="sunken", anchor="w")
        status_bar.pack(fill="x", side="bottom")

        # Main Rendering Canvas
        self.canvas = tk.Canvas(self.root, bg="#ffffff", highlightthickness=0)
        self.canvas.pack(fill="both", expand=True, side="top")

    def reload_page(self):
        if self.history and self.history_idx >= 0:
            self.load_url(self.history[self.history_idx])

    def _bind_events(self):
        self.canvas.bind("<Configure>", lambda e: self.render())
        self.canvas.bind("<Button-1>", self._on_canvas_click)
        self.canvas.bind("<Motion>", self._on_canvas_motion)
        self.root.bind("<Key>", self._on_key_press)
        self.root.bind("<Up>", lambda e: self.scroll(-SCROLL_STEP))
        self.root.bind("<Down>", lambda e: self.scroll(SCROLL_STEP))
        self.root.bind("<MouseWheel>", lambda e: self.scroll(-int(e.delta / 2)))

    def _on_canvas_click(self, event):
        click_x = event.x
        click_y = event.y + self.scroll_y

        if self.focused_input:
            self.focused_input.is_focused = False
            self.focused_input = None

        for cmd in self.display_list:
            if isinstance(cmd, DrawInput):
                if cmd.x <= click_x <= cmd.x + cmd.width and cmd.y <= click_y <= cmd.y + cmd.height:
                    cmd.is_focused = True
                    self.focused_input = cmd
                    self.render()
                    return

            if isinstance(cmd, DrawButton):
                if cmd.x <= click_x <= cmd.x + cmd.width and cmd.y <= click_y <= cmd.y + cmd.height:
                    query = self.focused_input.value if self.focused_input else ""
                    if query:
                        search_url = f"https://html.duckduckgo.com/html/?q={query}"
                        self.load_url(search_url)
                    return

            if hasattr(cmd, "href") and cmd.href:
                if cmd.x <= click_x <= cmd.x + cmd.width and cmd.y <= click_y <= cmd.y + cmd.height:
                    target = self.current_url_obj.resolve(cmd.href) if self.current_url_obj else cmd.href
                    self.load_url(target)
                    return

        self.render()

    def _on_key_press(self, event):
        if not self.focused_input:
            return

        if event.keysym == "BackSpace":
            self.focused_input.value = self.focused_input.value[:-1]
            if hasattr(self.focused_input, "node") and self.focused_input.node:
                self.focused_input.node.attributes["value"] = self.focused_input.value
            self.render()
        elif event.keysym == "Return":
            query = self.focused_input.value.strip()
            if query:
                search_url = f"https://html.duckduckgo.com/html/?q={query}"
                self.load_url(search_url)
        elif len(event.char) == 1 and ord(event.char) >= 32:
            self.focused_input.value += event.char
            if hasattr(self.focused_input, "node") and self.focused_input.node:
                self.focused_input.node.attributes["value"] = self.focused_input.value
            self.render()

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
            layout_root = build_layout_tree(dom_root, self.current_url_obj)

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
