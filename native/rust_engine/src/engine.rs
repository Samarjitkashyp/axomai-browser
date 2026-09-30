use crate::css_parser::{style_tree, CSSParser, Rule, DEFAULT_UA_STYLES};
use crate::html_parser::{HTMLParser, NodePtr, NodeType};
use crate::js_engine::V8JSEngine;
use crate::layout::{build_layout_tree, LayoutBox};
use crate::network::{base64_encode, URL};
use crate::painter::{build_display_list, DisplayCommand};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

static NEXT_ENGINE_ID: AtomicUsize = AtomicUsize::new(1);

pub struct NavigationResponse {
    pub engine_id: usize,
    pub document_id: u64,
    pub url: URL,
    pub body: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    Classic, // Standard external script (strict sequential document order)
    Async,   // async attribute (execute as soon as available out of order)
    Defer,   // defer attribute (execute in document order after DOM parsing)
    Module,  // type="module"
}

pub struct PendingScript {
    pub document_id: u64,
    pub order: usize,
    pub kind: ScriptKind,
    pub url: String,
    pub code: String,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub scroll_y: f32,
}

pub struct AxomaiEngine {
    pub engine_id: usize,
    pub document_id: u64,
    pub current_url: Option<URL>,
    pub current_title: String,
    pub dom_root: Option<NodePtr>,
    pub layout_root: Option<LayoutBox>,
    pub display_list: Vec<DisplayCommand>,
    pub scroll_y: f32,
    pub max_scroll_y: f32,
    pub focused_input_idx: Option<usize>,
    pub status_message: String,
    pub js_engine: V8JSEngine,
    pub active_css_rules: Vec<Rule>,
    pub is_dirty: bool,
    pub history: Vec<HistoryEntry>,
    pub history_index: usize,
    nav_tx: Sender<NavigationResponse>,
    nav_rx: Receiver<NavigationResponse>,
    script_tx: Sender<PendingScript>,
    script_rx: Receiver<PendingScript>,
    pub pending_scripts: usize,
    pub pending_defer_scripts: usize,
    pub next_ordered_script_to_run: usize,
    pub ordered_scripts_buffer: HashMap<usize, PendingScript>,
    pub next_ordered_defer_to_run: usize,
    pub ordered_defer_buffer: HashMap<usize, PendingScript>,
    pub dom_parsing_complete: bool,
    pub dom_content_loaded_dispatched: bool,
    pub load_dispatched: bool,
}

impl AxomaiEngine {
    pub fn new() -> Self {
        let engine_id = NEXT_ENGINE_ID.fetch_add(1, Ordering::SeqCst);
        let mut js_engine = V8JSEngine::new();
        js_engine.engine_id = engine_id;

        let (nav_tx, nav_rx) = channel();
        let (script_tx, script_rx) = channel();

        AxomaiEngine {
            engine_id,
            document_id: 1,
            current_url: None,
            current_title: "Axomai Browser".to_string(),
            dom_root: None,
            layout_root: None,
            display_list: Vec::new(),
            scroll_y: 0.0,
            max_scroll_y: 0.0,
            focused_input_idx: None,
            status_message: "Ready".to_string(),
            js_engine,
            active_css_rules: Vec::new(),
            is_dirty: true,
            history: Vec::new(),
            history_index: 0,
            nav_tx,
            nav_rx,
            script_tx,
            script_rx,
            pending_scripts: 0,
            pending_defer_scripts: 0,
            next_ordered_script_to_run: 0,
            ordered_scripts_buffer: HashMap::new(),
            next_ordered_defer_to_run: 0,
            ordered_defer_buffer: HashMap::new(),
            dom_parsing_complete: false,
            dom_content_loaded_dispatched: false,
            load_dispatched: false,
        }
    }

    /// Load URL with non-blocking asynchronous network fetching for http/https
    pub fn load_url(&mut self, url_str: &str, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        let url = URL::parse(url_str)?;
        self.current_url = Some(url.clone());
        self.scroll_y = 0.0;

        // Local or inline schemes load synchronously
        if url.scheme == "data" || url.scheme == "file" || url_str.starts_with("about:") {
            let (_headers, body) = url.request();
            return self.load_html(&body, viewport_w, viewport_h);
        }

        // Increment document lifecycle generation ID to cancel any in-flight navigation/scripts from prior page
        self.document_id += 1;
        let document_id = self.document_id;
        let engine_id = self.engine_id;
        let nav_tx = self.nav_tx.clone();
        let url_clone = url.clone();

        self.status_message = format!("Connecting to {}...", url.host);
        thread::spawn(move || {
            let (_headers, body) = url_clone.request();
            let _ = nav_tx.send(NavigationResponse {
                engine_id,
                document_id,
                url: url_clone,
                body,
                error: None,
            });
        });

        Ok(())
    }

    pub fn load_html(&mut self, html_content: &str, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        // Increment document lifecycle generation ID for new document
        self.document_id += 1;
        let doc_id = self.document_id;

        // Reset script execution ordering state & scrolling for the new document
        self.next_ordered_script_to_run = 0;
        self.ordered_scripts_buffer.clear();
        self.pending_scripts = 0;
        self.scroll_y = 0.0;
        self.is_dirty = true;
        self.dom_content_loaded_dispatched = false;
        self.load_dispatched = false;

        let url_str = self
            .current_url
            .as_ref()
            .map(|u| u.as_string())
            .unwrap_or_else(|| "about:blank".to_string());

        let parser = HTMLParser::new(html_content);
        let dom_root = parser.get_root();
        self.dom_root = Some(Rc::clone(&dom_root));

        // 1. Establish ONE persistent V8 Context bound to the root DOM BEFORE parsing begins
        self.js_engine.reset_page_context(Some(&dom_root), &url_str);

        let mut defer_script_counter = 0;
        let current_url = self.current_url.clone();
        let script_tx = self.script_tx.clone();

        // 2. Stream-driven HTML parse with synchronous Parser Pause -> Execute -> Resume
        let final_dom_root = parser.parse_interactive(|attrs, body, _tokenizer| {
            let is_async = attrs.contains_key("async");
            let is_defer = attrs.contains_key("defer");
            let script_type = attrs.get("type").map(|s| s.as_str()).unwrap_or("");
            let is_module = script_type == "module";

            let kind = if is_async {
                ScriptKind::Async
            } else if is_defer {
                ScriptKind::Defer
            } else if is_module {
                ScriptKind::Module
            } else {
                ScriptKind::Classic
            };

            // External script handling
            if let Some(src) = attrs.get("src") {
                if !src.trim().is_empty() {
                    let resolved_url = if let Some(ref current) = current_url {
                        current.resolve(src.trim())
                    } else {
                        src.trim().to_string()
                    };

                    if kind == ScriptKind::Defer || kind == ScriptKind::Module {
                        let defer_order = defer_script_counter;
                        defer_script_counter += 1;
                        self.pending_defer_scripts += 1;
                        self.pending_scripts += 1;

                        if let Ok(url_obj) = URL::parse(&resolved_url) {
                            if url_obj.scheme == "file" || url_obj.scheme == "data" {
                                let (_headers, js_code) = url_obj.request();
                                self.pending_defer_scripts -= 1;
                                self.pending_scripts -= 1;
                                self.ordered_defer_buffer.insert(
                                    defer_order,
                                    PendingScript {
                                        document_id: doc_id,
                                        order: defer_order,
                                        kind,
                                        url: resolved_url,
                                        code: js_code,
                                    },
                                );
                            } else {
                                println!(
                                    "[Axomai ScriptScheduler] Queued external {:?} script (doc_id {}, defer_order {}): {}",
                                    kind, doc_id, defer_order, resolved_url
                                );
                                let tx = script_tx.clone();
                                let script_url = resolved_url.clone();
                                thread::spawn(move || {
                                    let js_code = match ureq::get(&script_url).timeout(Duration::from_secs(5)).call() {
                                        Ok(resp) => resp.into_string().unwrap_or_default(),
                                        Err(e) => {
                                            eprintln!("[Axomai ScriptScheduler] Script fetch failed for {}: {}", script_url, e);
                                            String::new()
                                        }
                                    };
                                    let _ = tx.send(PendingScript {
                                        document_id: doc_id,
                                        order: defer_order,
                                        kind,
                                        url: script_url,
                                        code: js_code,
                                    });
                                });
                            }
                        }
                    } else if kind == ScriptKind::Async {
                        self.pending_scripts += 1;
                        println!("[Axomai ScriptScheduler] Queued external ASYNC script: {}", resolved_url);
                        let tx = script_tx.clone();
                        let script_url = resolved_url.clone();
                        thread::spawn(move || {
                            let js_code = match ureq::get(&script_url).timeout(Duration::from_secs(5)).call() {
                                Ok(resp) => resp.into_string().unwrap_or_default(),
                                Err(e) => {
                                    eprintln!("[Axomai ScriptScheduler] Async script fetch failed for {}: {}", script_url, e);
                                    String::new()
                                }
                            };
                            let _ = tx.send(PendingScript {
                                document_id: doc_id,
                                order: 0,
                                kind: ScriptKind::Async,
                                url: script_url,
                                code: js_code,
                            });
                        });
                    } else {
                        // Classic external script: GENUINE SYNCHRONOUS PARSER BLOCKING FETCH + EXECUTE
                        println!("[Axomai ParserController] Synchronous parser pause -> fetching blocking classic script: {}", resolved_url);
                        let js_code = if let Ok(url_obj) = URL::parse(&resolved_url) {
                            if url_obj.scheme == "file" || url_obj.scheme == "data" {
                                let (_headers, code) = url_obj.request();
                                code
                            } else {
                                match ureq::get(&resolved_url).timeout(Duration::from_secs(5)).call() {
                                    Ok(resp) => resp.into_string().unwrap_or_default(),
                                    Err(e) => {
                                        eprintln!("[Axomai ParserController] Blocking script fetch failed for {}: {}", resolved_url, e);
                                        String::new()
                                    }
                                }
                            }
                        } else {
                            String::new()
                        };
                        if !js_code.is_empty() {
                            println!("[Axomai ParserController] Executing blocking classic script ({} bytes) -> live DOM mutation", js_code.len());
                            let _ = self.js_engine.execute(&js_code);
                            let injected_html = self.js_engine.take_written_html();
                            if !injected_html.is_empty() {
                                println!("[Axomai ParserController] document.write injected {} bytes into tokenizer stream", injected_html.len());
                                _tokenizer.insert_input(&injected_html);
                            }
                            println!("[Axomai ParserController] Blocking classic script execution complete -> parser resumed");
                        }
                    }
                    return;
                }
            }

            // Inline script handling
            if !body.trim().is_empty() {
                if kind == ScriptKind::Defer || kind == ScriptKind::Module {
                    let defer_order = defer_script_counter;
                    defer_script_counter += 1;
                    self.ordered_defer_buffer.insert(
                        defer_order,
                        PendingScript {
                            document_id: doc_id,
                            order: defer_order,
                            kind,
                            url: if kind == ScriptKind::Module { "inline-module".to_string() } else { "inline-defer".to_string() },
                            code: body.to_string(),
                        },
                    );
                } else {
                    // GENUINE SYNCHRONOUS PARSER PAUSE -> EXECUTE -> RESUME
                    println!("[Axomai ParserController] Synchronous parser pause -> executing inline classic script ({} bytes)", body.len());
                    let _ = self.js_engine.execute(body);
                    let injected_html = self.js_engine.take_written_html();
                    if !injected_html.is_empty() {
                        println!("[Axomai ParserController] document.write injected {} bytes into tokenizer stream", injected_html.len());
                        _tokenizer.insert_input(&injected_html);
                    }
                    println!("[Axomai ParserController] Inline classic script execution complete -> parser resumed");
                }
            }
        });

        self.dom_root = Some(Rc::clone(&final_dom_root));
        self.dom_parsing_complete = true;

        // 3. Drain all currently available defer/module scripts in strict document order
        while let Some(defer_script) = self.ordered_defer_buffer.remove(&self.next_ordered_defer_to_run) {
            if !defer_script.code.is_empty() {
                println!(
                    "[Axomai ScriptController] Executing deferred/module script (defer_order {}): {}",
                    self.next_ordered_defer_to_run, defer_script.url
                );
                let code_to_run = if defer_script.kind == ScriptKind::Module {
                    format!("(function() {{\n'use strict';\n{}\n}})();", defer_script.code)
                } else {
                    defer_script.code
                };
                let _ = self.js_engine.execute(&code_to_run);
            }
            self.next_ordered_defer_to_run += 1;
        }

        // 4. Dispatch DOMContentLoaded IF AND ONLY IF all defer/module scripts have finished executing!
        if self.pending_defer_scripts == 0 {
            println!("[Axomai Lifecycle] All defer/module scripts finished -> Dispatching DOMContentLoaded");
            self.js_engine.dispatch_dom_content_loaded();
            self.dom_content_loaded_dispatched = true;
        } else {
            println!(
                "[Axomai Lifecycle] Parser complete but {} external defer/module script(s) still in-flight -> DOMContentLoaded gated",
                self.pending_defer_scripts
            );
        }

        // 6. Extract <title> and update current_title & history stack
        let mut title_buf = String::new();
        extract_title_tag(&final_dom_root, &mut title_buf);
        let extracted_title = if !title_buf.trim().is_empty() {
            title_buf.trim().to_string()
        } else if let Some(ref u) = self.current_url {
            u.host.clone()
        } else {
            "Axomai Browser".to_string()
        };
        self.current_title = extracted_title.clone();

        // Update history entry if not already matching current top of history
        let current_url_str = self.current_url.as_ref().map(|u| u.as_string()).unwrap_or_else(|| "about:blank".to_string());
        if self.history.is_empty() {
            self.history.push(HistoryEntry {
                url: current_url_str,
                title: extracted_title,
                scroll_y: 0.0,
            });
            self.history_index = 0;
        } else if self.history[self.history_index].url != current_url_str {
            if self.history_index + 1 < self.history.len() {
                self.history.truncate(self.history_index + 1);
            }
            self.history.push(HistoryEntry {
                url: current_url_str,
                title: extracted_title,
                scroll_y: 0.0,
            });
            self.history_index = self.history.len() - 1;
        } else {
            // Same URL (e.g. reload or back/forward traversal), update title
            self.history[self.history_index].title = extracted_title;
        }

        // 4. Extract <style> & Style Tree
        let mut author_css_list = Vec::new();
        extract_style_tags(&dom_root, &mut author_css_list);
        let author_css = author_css_list.join("\n");

        let ua_rules = CSSParser::new(DEFAULT_UA_STYLES).parse();
        let author_rules = CSSParser::new(&author_css).parse();
        let mut all_rules = ua_rules;
        all_rules.extend(author_rules);

        self.active_css_rules = all_rules;
        self.dom_root = Some(dom_root);

        // 5. Restyle, Layout & Display List
        self.restyle_and_relayout(viewport_w, viewport_h);

        // If no pending external scripts, dispatch load event
        if self.pending_scripts == 0 {
            self.js_engine.dispatch_load_event();
            self.load_dispatched = true;
        }

        self.status_message = "Page Loaded Successfully".to_string();
        Ok(())
    }

    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        !self.history.is_empty() && self.history_index + 1 < self.history.len()
    }

    pub fn go_back(&mut self, viewport_w: f32, viewport_h: f32) -> Option<String> {
        if self.can_go_back() {
            // Save current scroll position in current entry before navigating back
            if self.history_index < self.history.len() {
                self.history[self.history_index].scroll_y = self.scroll_y;
            }
            self.history_index -= 1;
            let entry = self.history[self.history_index].clone();
            let saved_scroll = entry.scroll_y;
            let _ = self.load_url(&entry.url, viewport_w, viewport_h);
            self.scroll_y = saved_scroll;
            self.restyle_and_relayout(viewport_w, viewport_h);
            return Some(entry.url);
        }
        None
    }

    pub fn go_forward(&mut self, viewport_w: f32, viewport_h: f32) -> Option<String> {
        if self.can_go_forward() {
            // Save current scroll position in current entry before navigating forward
            if self.history_index < self.history.len() {
                self.history[self.history_index].scroll_y = self.scroll_y;
            }
            self.history_index += 1;
            let entry = self.history[self.history_index].clone();
            let saved_scroll = entry.scroll_y;
            let _ = self.load_url(&entry.url, viewport_w, viewport_h);
            self.scroll_y = saved_scroll;
            self.restyle_and_relayout(viewport_w, viewport_h);
            return Some(entry.url);
        }
        None
    }

    pub fn reload(&mut self, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        if let Some(ref u) = self.current_url {
            let url_str = u.as_string();
            let saved_scroll = self.scroll_y;
            let res = self.load_url(&url_str, viewport_w, viewport_h);
            self.scroll_y = saved_scroll;
            res
        } else {
            Ok(())
        }
    }

    /// Recalculates style using both UA and Author CSS rules, and reconstructs the layout and display list
    pub fn restyle_and_relayout(&mut self, viewport_w: f32, viewport_h: f32) {
        if let Some(ref dom_root) = self.dom_root {
            // 1. Re-apply cascading style tree with all active rules (UA + Author)
            style_tree(dom_root, &self.active_css_rules);

            // 2. Rebuild Layout Tree
            if let Some(mut layout_box) = build_layout_tree(dom_root, self.current_url.as_ref()) {
                let total_h = layout_box.layout(0.0, 0.0, viewport_w.max(800.0));
                self.max_scroll_y = (total_h - viewport_h).max(0.0);
                self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y);

                // 3. Rebuild Display List
                let mut list = Vec::new();
                build_display_list(&layout_box, &mut list);

                self.layout_root = Some(layout_box);
                self.display_list = list;
            } else {
                self.layout_root = None;
                self.display_list.clear();
                self.max_scroll_y = 0.0;
                self.scroll_y = 0.0;
            }
            self.is_dirty = true;
        }
    }

    pub fn handle_scroll(&mut self, delta_y: f32) -> bool {
        let old_scroll = self.scroll_y;
        self.scroll_y = (self.scroll_y + delta_y).clamp(0.0, self.max_scroll_y);
        let changed = (old_scroll - self.scroll_y).abs() > 0.1;
        if changed {
            if self.history_index < self.history.len() {
                self.history[self.history_index].scroll_y = self.scroll_y;
            }
            self.is_dirty = true;
        }
        changed
    }

    pub fn handle_click(&mut self, click_x: f32, click_y: f32, scroll_y: f32) -> Option<String> {
        let effective_scroll = if scroll_y != 0.0 { scroll_y } else { self.scroll_y };
        let abs_y = click_y + effective_scroll;
        let query = self.get_focused_input_val();

        // Unfocus previous input
        if let Some(idx) = self.focused_input_idx {
            if idx < self.display_list.len() {
                if let DisplayCommand::DrawInput { ref mut is_focused, .. } = self.display_list[idx] {
                    *is_focused = false;
                }
            }
            self.focused_input_idx = None;
            self.is_dirty = true;
        }

        for (idx, cmd) in self.display_list.iter_mut().enumerate() {
            match cmd {
                DisplayCommand::DrawInput {
                    x,
                    y,
                    width,
                    height,
                    ref mut is_focused,
                    ..
                } => {
                    if click_x >= *x && click_x <= *x + *width && abs_y >= *y && abs_y <= *y + *height {
                        *is_focused = true;
                        self.focused_input_idx = Some(idx);
                        self.is_dirty = true;
                        self.js_engine.dispatch_click_event("input", click_x, abs_y);
                        return None;
                    }
                }

                DisplayCommand::DrawButton {
                    x, y, width, height, ..
                } => {
                    if click_x >= *x && click_x <= *x + *width && abs_y >= *y && abs_y <= *y + *height {
                        self.js_engine.dispatch_click_event("button", click_x, abs_y);
                        if !query.is_empty() {
                            return Some(format!("https://html.duckduckgo.com/html/?q={}", query));
                        }
                    }
                }

                DisplayCommand::DrawText {
                    x,
                    y,
                    width,
                    height,
                    ref href,
                    ..
                } => {
                    if !href.is_empty()
                        && click_x >= *x
                        && click_x <= *x + *width
                        && abs_y >= *y
                        && abs_y <= *y + *height
                    {
                        self.js_engine.dispatch_click_event("a", click_x, abs_y);
                        if let Some(ref url_obj) = self.current_url {
                            return Some(url_obj.resolve(href));
                        } else {
                            return Some(href.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        None
    }

    pub fn handle_key(&mut self, key_str: &str) -> Option<String> {
        if let Some(idx) = self.focused_input_idx {
            if idx < self.display_list.len() {
                if let DisplayCommand::DrawInput {
                    ref mut value, ..
                } = self.display_list[idx]
                {
                    if key_str == "BackSpace" || key_str == "\u{8}" {
                        value.pop();
                        self.is_dirty = true;
                    } else if key_str == "Return" || key_str == "Enter" || key_str == "\r" || key_str == "\n" {
                        let query = value.trim().to_string();
                        if !query.is_empty() {
                            return Some(format!("https://html.duckduckgo.com/html/?q={}", query));
                        }
                    } else if key_str.len() == 1 {
                        value.push_str(key_str);
                        self.is_dirty = true;
                    }
                }
            }
        }
        None
    }

    pub fn handle_hover(&self, hover_x: f32, hover_y: f32, scroll_y: f32) -> Option<String> {
        let effective_scroll = if scroll_y != 0.0 { scroll_y } else { self.scroll_y };
        let abs_y = hover_y + effective_scroll;
        for cmd in &self.display_list {
            if let DisplayCommand::DrawText {
                x,
                y,
                width,
                height,
                ref href,
                ..
            } = cmd
            {
                if !href.is_empty()
                    && hover_x >= *x
                    && hover_x <= *x + *width
                    && abs_y >= *y
                    && abs_y <= *y + *height
                {
                    if let Some(ref url_obj) = self.current_url {
                        return Some(url_obj.resolve(href));
                    } else {
                        return Some(href.clone());
                    }
                }
            }
        }
        None
    }

    fn get_focused_input_val(&self) -> String {
        if let Some(idx) = self.focused_input_idx {
            if idx < self.display_list.len() {
                if let DisplayCommand::DrawInput { ref value, .. } = self.display_list[idx] {
                    return value.clone();
                }
            }
        }
        String::new()
    }

    pub fn process_event_loop(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        let mut executed = false;

        // 1. Process completed non-blocking page navigations via per-engine channel with document generation filter
        while let Ok(nav) = self.nav_rx.try_recv() {
            if nav.document_id != self.document_id {
                println!("[Axomai Engine] Discarding stale navigation response (doc_id {} vs current {})", nav.document_id, self.document_id);
                continue;
            }
            let saved_scroll = if self.history_index < self.history.len() && self.history[self.history_index].url == nav.url.as_string() {
                self.history[self.history_index].scroll_y
            } else {
                0.0
            };
            self.current_url = Some(nav.url);
            let _ = self.load_html(&nav.body, viewport_w, viewport_h);
            if saved_scroll > 0.0 {
                self.scroll_y = saved_scroll;
                self.restyle_and_relayout(viewport_w, viewport_h);
            }
            self.status_message = "Page Loaded Successfully".to_string();
            executed = true;
        }

        // 2. Process JS engine event loop (timers, async fetch, microtasks)
        let js_executed = self.js_engine.process_event_loop();
        if js_executed {
            // Reapply author + UA styles and recalculate layout
            self.restyle_and_relayout(viewport_w, viewport_h);
            executed = true;
        }

        // 3. Process completed external scripts via async ScriptScheduler with generation tracking & strict ordering
        let mut script_mutated = false;
        let mut async_scripts = Vec::new();

        while let Ok(script) = self.script_rx.try_recv() {
            if script.document_id != self.document_id {
                println!(
                    "[Axomai ScriptScheduler] Discarding obsolete script from previous document (doc_id {} vs current {})",
                    script.document_id, self.document_id
                );
                continue;
            }
            if self.pending_scripts > 0 {
                self.pending_scripts -= 1;
            }
            if script.kind == ScriptKind::Defer || script.kind == ScriptKind::Module {
                if self.pending_defer_scripts > 0 {
                    self.pending_defer_scripts -= 1;
                }
                self.ordered_defer_buffer.insert(script.order, script);
            } else if script.kind == ScriptKind::Async {
                async_scripts.push(script);
            } else {
                self.ordered_scripts_buffer.insert(script.order, script);
            }
        }

        // Execute async scripts immediately as they arrive
        for script in async_scripts {
            if !script.code.is_empty() {
                println!("[Axomai ScriptScheduler] Executing async script: {}", script.url);
                if let Ok(mutated) = self.js_engine.execute(&script.code) {
                    if mutated {
                        script_mutated = true;
                    }
                }
            }
            executed = true;
        }

        // If DOM parsing is complete, execute deferred/module scripts in strict document order
        if self.dom_parsing_complete {
            while let Some(defer_script) = self.ordered_defer_buffer.remove(&self.next_ordered_defer_to_run) {
                if !defer_script.code.is_empty() {
                    println!(
                        "[Axomai ScriptScheduler] Executing deferred/module script (defer_order {}): {}",
                        self.next_ordered_defer_to_run, defer_script.url
                    );
                    let code_to_run = if defer_script.kind == ScriptKind::Module {
                        format!("(function() {{\n'use strict';\n{}\n}})();", defer_script.code)
                    } else {
                        defer_script.code
                    };
                    if let Ok(mutated) = self.js_engine.execute(&code_to_run) {
                        if mutated {
                            script_mutated = true;
                        }
                    }
                }
                self.next_ordered_defer_to_run += 1;
                executed = true;
            }

            // Gated DOMContentLoaded dispatch: fires once ALL defer/module scripts have completed!
            if self.pending_defer_scripts == 0 && !self.dom_content_loaded_dispatched {
                println!("[Axomai Lifecycle] In-flight defer/module scripts completed -> Dispatching DOMContentLoaded");
                self.js_engine.dispatch_dom_content_loaded();
                self.dom_content_loaded_dispatched = true;
                script_mutated = true;
                executed = true;
            }
        }

        // Execute sequential ordered scripts strictly in order (0 -> 1 -> 2 -> ...)
        while let Some(script) = self.ordered_scripts_buffer.remove(&self.next_ordered_script_to_run) {
            if !script.code.is_empty() {
                println!(
                    "[Axomai ScriptScheduler] Executing ordered script (order {}): {}",
                    self.next_ordered_script_to_run, script.url
                );
                if let Ok(mutated) = self.js_engine.execute(&script.code) {
                    if mutated {
                        script_mutated = true;
                    }
                }
            }
            self.next_ordered_script_to_run += 1;
            executed = true;
        }

        if script_mutated {
            self.restyle_and_relayout(viewport_w, viewport_h);
        }

        // Load event fires once DOMContentLoaded has fired AND all scripts (including async) have completed!
        if self.pending_scripts == 0 && self.dom_content_loaded_dispatched && !self.load_dispatched {
            println!("[Axomai Lifecycle] All scripts & subresources complete -> Dispatching load event");
            self.js_engine.dispatch_load_event();
            self.load_dispatched = true;
            executed = true;
        }

        let was_dirty = self.is_dirty;
        self.is_dirty = false;

        executed || was_dirty
    }

    /// Serialize current display list commands to JSON for desktop WebView/canvas rendering bridge with scroll offset
    pub fn get_display_list_json(&self) -> String {
        let mut json = String::from("[");
        let scroll_y = self.scroll_y;
        for (i, cmd) in self.display_list.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            match cmd {
                DisplayCommand::DrawRect { x1, y1, x2, y2, color } => {
                    json.push_str(&format!(
                        r#"{{"type":"rect","x1":{},"y1":{},"x2":{},"y2":{},"color":"{}"}}"#,
                        x1, y1 - scroll_y, x2, y2 - scroll_y, color
                    ));
                }
                DisplayCommand::DrawText {
                    x,
                    y,
                    width,
                    height,
                    text,
                    font_size,
                    font_weight,
                    font_style,
                    color,
                    href,
                } => {
                    let escaped_text = text.replace('\\', "\\\\").replace('"', "\\\"");
                    json.push_str(&format!(
                        r#"{{"type":"text","x":{},"y":{},"width":{},"height":{},"text":"{}","fontSize":{},"fontWeight":"{}","fontStyle":"{}","color":"{}","href":"{}"}}"#,
                        x, y - scroll_y, width, height, escaped_text, font_size, font_weight, font_style, color, href
                    ));
                }
                DisplayCommand::DrawInput {
                    x,
                    y,
                    width,
                    height,
                    value,
                    placeholder,
                    is_focused,
                } => {
                    json.push_str(&format!(
                        r#"{{"type":"input","x":{},"y":{},"width":{},"height":{},"value":"{}","placeholder":"{}","isFocused":{}}}"#,
                        x, y - scroll_y, width, height, value, placeholder, is_focused
                    ));
                }
                DisplayCommand::DrawButton {
                    x,
                    y,
                    width,
                    height,
                    label,
                } => {
                    json.push_str(&format!(
                        r#"{{"type":"button","x":{},"y":{},"width":{},"height":{},"label":"{}"}}"#,
                        x, y - scroll_y, width, height, label
                    ));
                }
                DisplayCommand::DrawImage {
                    x,
                    y,
                    width,
                    height,
                    image_bytes,
                } => {
                    let b64 = if !image_bytes.is_empty() {
                        format!("data:image/png;base64,{}", base64_encode(image_bytes))
                    } else {
                        String::new()
                    };
                    json.push_str(&format!(
                        r#"{{"type":"image","x":{},"y":{},"width":{},"height":{},"src":"{}"}}"#,
                        x, y - scroll_y, width, height, b64
                    ));
                }
            }
        }
        json.push(']');
        json
    }

    /// Check if there are pending async fetch responses, background navigations, or timers ready for event loop processing
    pub fn has_pending_events(&self) -> bool {
        self.is_dirty
            || self.pending_scripts > 0
            || self.pending_defer_scripts > 0
            || !self.ordered_scripts_buffer.is_empty()
            || !self.ordered_defer_buffer.is_empty()
            || self.js_engine.has_pending_events()
    }
}

fn extract_style_tags(node: &NodePtr, css_list: &mut Vec<String>) {
    let b = node.borrow();
    if let NodeType::Element { ref tag, .. } = b.node_type {
        if tag == "style" {
            for child in &b.children {
                let cb = child.borrow();
                if let NodeType::Text { ref text } = cb.node_type {
                    css_list.push(text.clone());
                }
            }
        }
    }
    for child in &b.children {
        extract_style_tags(child, css_list);
    }
}

#[derive(Debug, Clone)]
pub enum ScriptEntry {
    Inline { code: String, kind: ScriptKind },
    External { src: String, kind: ScriptKind },
}

fn extract_script_tags(node: &NodePtr, script_list: &mut Vec<ScriptEntry>) {
    let b = node.borrow();
    if let NodeType::Element {
        ref tag,
        ref attributes,
        ..
    } = b.node_type
    {
        if tag == "script" {
            let is_async = attributes.contains_key("async");
            let is_defer = attributes.contains_key("defer");
            let script_type = attributes.get("type").map(|s| s.as_str()).unwrap_or("");
            let is_module = script_type == "module";

            let kind = if is_async {
                ScriptKind::Async
            } else if is_defer {
                ScriptKind::Defer
            } else if is_module {
                ScriptKind::Module
            } else {
                ScriptKind::Classic
            };

            if let Some(src) = attributes.get("src") {
                if !src.trim().is_empty() {
                    script_list.push(ScriptEntry::External {
                        src: src.trim().to_string(),
                        kind,
                    });
                    return;
                }
            }
            let mut inline_code = String::new();
            for child in &b.children {
                let cb = child.borrow();
                if let NodeType::Text { ref text } = cb.node_type {
                    inline_code.push_str(text);
                }
            }
            if !inline_code.trim().is_empty() {
                script_list.push(ScriptEntry::Inline {
                    code: inline_code,
                    kind,
                });
            }
        }
    }
    for child in &b.children {
        extract_script_tags(child, script_list);
    }
}

fn extract_title_tag(node: &NodePtr, title: &mut String) {
    let b = node.borrow();
    if let NodeType::Element { ref tag, .. } = b.node_type {
        if tag == "title" {
            for child in &b.children {
                let cb = child.borrow();
                if let NodeType::Text { ref text } = cb.node_type {
                    title.push_str(text);
                }
            }
            return;
        }
    }
    for child in &b.children {
        extract_title_tag(child, title);
    }
}



