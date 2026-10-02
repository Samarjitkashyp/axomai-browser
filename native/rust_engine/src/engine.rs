use crate::css_parser::{
    parse_animation_shorthand, style_tree,
    CSSParser, KeyframeAnimation, Rule, DEFAULT_UA_STYLES,
};
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
use std::time::{Duration, Instant};

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
    pub active_keyframes: HashMap<String, KeyframeAnimation>,
    pub animation_start_time: Option<Instant>,
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
    pub is_dragging_scrollbar: bool,
    pub drag_is_horizontal: bool,
    pub drag_start_x: f32,
    pub drag_start_y: f32,
    pub drag_initial_scroll: f32,
    pub drag_initial_scroll_x: f32,
    pub drag_container_pos: Option<(f32, f32)>,
    pub adblock_engine: crate::adblock_engine::AdBlockEngine,
    pub theme_engine: crate::theme_engine::ThemeEngine,
    pub ai_assistant: crate::ai_assistant::AiAssistant,
    pub reader_mode_active: bool,
    pub reader_article: Option<crate::reader_mode::ReaderArticle>,
    pub page_summary: Option<String>,
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
            active_keyframes: HashMap::new(),
            animation_start_time: Some(Instant::now()),
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
            is_dragging_scrollbar: false,
            drag_is_horizontal: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_initial_scroll: 0.0,
            drag_initial_scroll_x: 0.0,
            drag_container_pos: None,
            adblock_engine: crate::adblock_engine::AdBlockEngine::new(),
            theme_engine: crate::theme_engine::ThemeEngine::new(),
            ai_assistant: crate::ai_assistant::AiAssistant::new(),
            reader_mode_active: false,
            reader_article: None,
            page_summary: None,
        }
    }

    /// Load URL with non-blocking asynchronous network fetching for http/https
    pub fn load_url(&mut self, url_str: &str, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        if self.adblock_engine.should_block_url(url_str) {
            self.status_message = format!("Blocked by Privacy Shield: {}", url_str);
            return Err(format!("URL blocked by AdBlockEngine: {}", url_str));
        }

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
        self.js_engine.set_parsing(true);

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

        self.js_engine.set_parsing(false);
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
                    let escaped = defer_script.code.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
                    format!("if (typeof window.__executeModule === 'function') {{ window.__executeModule(`{}`, `{}`); }}", escaped, defer_script.url)
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

        let ua_sheet = CSSParser::new(DEFAULT_UA_STYLES).parse_stylesheet();
        let author_sheet = CSSParser::new(&author_css).parse_stylesheet();
        let mut all_rules = ua_sheet.rules;
        all_rules.extend(author_sheet.rules);

        self.active_css_rules = all_rules;
        self.active_keyframes = author_sheet.keyframes;
        self.animation_start_time = Some(Instant::now());
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

                // Sync live computed geometry to V8 DOM registry
                let mut geom_map = HashMap::new();
                layout_box.collect_layout_boxes_geometry(&mut geom_map);
                self.js_engine.sync_layout_geometry(&geom_map);

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

    /// Advances active CSS @keyframes and transitions by elapsed time and triggers repainting
    pub fn tick_animations(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        if self.active_keyframes.is_empty() {
            return false;
        }
        let start = match self.animation_start_time {
            Some(t) => t,
            None => return false,
        };
        let elapsed_sec = start.elapsed().as_secs_f32();

        let mut mutated = false;
        if let Some(ref dom_root) = self.dom_root {
            apply_node_animations(dom_root, elapsed_sec, &self.active_keyframes, &mut mutated);
        }

        if mutated {
            self.restyle_and_relayout(viewport_w, viewport_h);
        }
        mutated
    }

    pub fn handle_scroll(&mut self, delta_y: f32) -> bool {
        self.handle_scroll_at(0.0, 0.0, 0.0, delta_y, 1200.0, 800.0)
    }

    pub fn handle_scroll_at(&mut self, cursor_x: f32, cursor_y: f32, delta_x: f32, delta_y: f32, _viewport_w: f32, _viewport_h: f32) -> bool {
        let abs_y = cursor_y + self.scroll_y;

        // 1. Try scrolling an inner scroll container if mouse is over one
        let mut scrolled_container = false;
        if let Some(ref mut layout_box) = self.layout_root {
            if let Some(sc) = layout_box.find_scroll_container_at_mut(cursor_x, abs_y) {
                let max_y = (sc.scroll_height - sc.height).max(0.0);
                let max_x = (sc.scroll_width - sc.width).max(0.0);
                let old_top = sc.scroll_top;
                let old_left = sc.scroll_left;

                if max_y > 0.0 && delta_y != 0.0 {
                    sc.scroll_top = (sc.scroll_top + delta_y).clamp(0.0, max_y);
                    if (sc.scroll_top - old_top).abs() > 0.1 {
                        scrolled_container = true;
                    }
                }
                if max_x > 0.0 && delta_x != 0.0 {
                    sc.scroll_left = (sc.scroll_left + delta_x).clamp(0.0, max_x);
                    if (sc.scroll_left - old_left).abs() > 0.1 {
                        scrolled_container = true;
                    }
                }
            }
        }

        if scrolled_container {
            // Rebuild display list with updated container scroll offsets
            if let Some(ref layout_box) = self.layout_root {
                let mut list = Vec::new();
                build_display_list(layout_box, &mut list);
                self.display_list = list;
                self.is_dirty = true;
            }
            return true;
        }

        // 2. Fallback to viewport scroll
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

        // 1. First perform tree-based hit test if layout tree exists
        if let Some(ref layout_box) = self.layout_root {
            if let Some(hit) = layout_box.hit_test(click_x, abs_y) {
                if !hit.href.is_empty() {
                    self.js_engine.dispatch_click_event("a", click_x, abs_y);
                    if let Some(ref url_obj) = self.current_url {
                        return Some(url_obj.resolve(&hit.href));
                    } else {
                        return Some(hit.href.clone());
                    }
                }
            }
        }

        // 2. Match interactive display commands
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

    pub fn handle_key_event(
        &mut self,
        event_type: &str,
        key_str: &str,
        code_str: &str,
        key_code: u32,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
        repeat: bool,
    ) -> Option<String> {
        let not_prevented = self.js_engine.dispatch_keyboard_event(
            event_type, key_str, code_str, key_code, ctrl, alt, shift, meta, repeat,
        );

        if (event_type == "keydown" || event_type == "keypress") && not_prevented {
            return self.handle_key(key_str);
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

    pub fn handle_pointer_down(&mut self, x: f32, y: f32, button: i32) -> Option<String> {
        let abs_y = y + self.scroll_y;

        // 1. Check if clicking vertical or horizontal scrollbar on an inner scroll container
        let mut clicked_scrollbar = false;
        if let Some(ref mut layout_box) = self.layout_root {
            if let Some(sc) = layout_box.find_scroll_container_at_mut(x, abs_y) {
                let v_track_x = sc.x + sc.width - 9.0;
                let h_track_y = sc.y + sc.height - 9.0;

                // Vertical scrollbar click
                if sc.scroll_height > sc.height && x >= v_track_x && x <= v_track_x + 9.0 && abs_y >= sc.y && abs_y <= sc.y + sc.height {
                    self.is_dragging_scrollbar = true;
                    self.drag_is_horizontal = false;
                    self.drag_start_y = y;
                    self.drag_initial_scroll = sc.scroll_top;
                    self.drag_container_pos = Some((x, abs_y));
                    clicked_scrollbar = true;
                }
                // Horizontal scrollbar click
                else if sc.scroll_width > sc.width && abs_y >= h_track_y && abs_y <= h_track_y + 9.0 && x >= sc.x && x <= sc.x + sc.width {
                    self.is_dragging_scrollbar = true;
                    self.drag_is_horizontal = true;
                    self.drag_start_x = x;
                    self.drag_initial_scroll_x = sc.scroll_left;
                    self.drag_container_pos = Some((x, abs_y));
                    clicked_scrollbar = true;
                }
            }
        }

        if !clicked_scrollbar {
            // Find target element selector from hit test
            let mut target_selector = "body".to_string();
            if let Some(ref layout_box) = self.layout_root {
                if let Some(hit) = layout_box.hit_test(x, abs_y) {
                    if !hit.id.is_empty() {
                        target_selector = format!("#{}", hit.id);
                    } else if !hit.class_name.is_empty() {
                        target_selector = format!(".{}", hit.class_name.split_whitespace().next().unwrap_or(""));
                    } else if !hit.tag_name.is_empty() {
                        target_selector = hit.tag_name.clone();
                    }
                }
            }
            self.js_engine.dispatch_pointer_event("pointerdown", &target_selector, x, y, button);
        }

        self.handle_click(x, y, 0.0)
    }

    pub fn handle_pointer_move(&mut self, x: f32, y: f32) -> Option<String> {
        let abs_y = y + self.scroll_y;

        // 1. If dragging scrollbar, update scroll position along active drag axis
        if self.is_dragging_scrollbar {
            if self.drag_is_horizontal {
                let dx = x - self.drag_start_x;
                if let Some((cx, cy)) = self.drag_container_pos {
                    if let Some(ref mut layout_box) = self.layout_root {
                        if let Some(sc) = layout_box.find_scroll_container_at_mut(cx, cy) {
                            let max_x = (sc.scroll_width - sc.width).max(1.0);
                            let ratio = sc.scroll_width / sc.width.max(1.0);
                            sc.scroll_left = (self.drag_initial_scroll_x + dx * ratio).clamp(0.0, max_x);
                        }
                    }
                    if let Some(ref layout_box) = self.layout_root {
                        let mut list = Vec::new();
                        build_display_list(layout_box, &mut list);
                        self.display_list = list;
                        self.is_dirty = true;
                    }
                }
            } else {
                let dy = y - self.drag_start_y;
                if let Some((cx, cy)) = self.drag_container_pos {
                    if let Some(ref mut layout_box) = self.layout_root {
                        if let Some(sc) = layout_box.find_scroll_container_at_mut(cx, cy) {
                            let max_y = (sc.scroll_height - sc.height).max(1.0);
                            let ratio = sc.scroll_height / sc.height.max(1.0);
                            sc.scroll_top = (self.drag_initial_scroll + dy * ratio).clamp(0.0, max_y);
                        }
                    }
                    if let Some(ref layout_box) = self.layout_root {
                        let mut list = Vec::new();
                        build_display_list(layout_box, &mut list);
                        self.display_list = list;
                        self.is_dirty = true;
                    }
                } else {
                    let ratio = (self.max_scroll_y + 800.0) / 800.0;
                    self.scroll_y = (self.drag_initial_scroll + dy * ratio).clamp(0.0, self.max_scroll_y);
                    self.is_dirty = true;
                }
            }
        }

        // 2. Dispatch pointermove to JS engine
        let mut target_selector = "body".to_string();
        if let Some(ref layout_box) = self.layout_root {
            if let Some(hit) = layout_box.hit_test(x, abs_y) {
                if !hit.id.is_empty() {
                    target_selector = format!("#{}", hit.id);
                } else if !hit.class_name.is_empty() {
                    target_selector = format!(".{}", hit.class_name.split_whitespace().next().unwrap_or(""));
                } else if !hit.tag_name.is_empty() {
                    target_selector = hit.tag_name.clone();
                }
            }
        }
        self.js_engine.dispatch_pointer_event("pointermove", &target_selector, x, y, 0);

        self.handle_hover(x, y, 0.0)
    }

    pub fn handle_pointer_up(&mut self, x: f32, y: f32, button: i32) -> Option<String> {
        self.is_dragging_scrollbar = false;
        self.drag_is_horizontal = false;
        self.drag_container_pos = None;

        let abs_y = y + self.scroll_y;
        let mut target_selector = "body".to_string();
        if let Some(ref layout_box) = self.layout_root {
            if let Some(hit) = layout_box.hit_test(x, abs_y) {
                if !hit.id.is_empty() {
                    target_selector = format!("#{}", hit.id);
                } else if !hit.class_name.is_empty() {
                    target_selector = format!(".{}", hit.class_name.split_whitespace().next().unwrap_or(""));
                } else if !hit.tag_name.is_empty() {
                    target_selector = hit.tag_name.clone();
                }
            }
        }
        self.js_engine.dispatch_pointer_event("pointerup", &target_selector, x, y, button);
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
                        let escaped = defer_script.code.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
                        format!("if (typeof window.__executeModule === 'function') {{ window.__executeModule(`{}`, `{}`); }}", escaped, defer_script.url)
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
                DisplayCommand::DrawRect { x1, y1, x2, y2, color, border_radius } => {
                    json.push_str(&format!(
                        r#"{{"type":"rect","x1":{},"y1":{},"x2":{},"y2":{},"color":"{}","borderRadius":{}}}"#,
                        x1, y1 - scroll_y, x2, y2 - scroll_y, color, border_radius
                    ));
                }
                DisplayCommand::DrawBoxShadow {
                    x,
                    y,
                    width,
                    height,
                    offset_x,
                    offset_y,
                    blur_radius,
                    spread_radius,
                    color,
                    border_radius,
                    is_inset,
                } => {
                    json.push_str(&format!(
                        r#"{{"type":"boxShadow","x":{},"y":{},"width":{},"height":{},"offsetX":{},"offsetY":{},"blur":{},"spread":{},"color":"{}","borderRadius":{},"isInset":{}}}"#,
                        x, y - scroll_y, width, height, offset_x, offset_y, blur_radius, spread_radius, color, border_radius, is_inset
                    ));
                }
                DisplayCommand::PushClip { x, y, width, height, border_radius } => {
                    json.push_str(&format!(
                        r#"{{"type":"pushClip","x":{},"y":{},"width":{},"height":{},"borderRadius":{}}}"#,
                        x, y - scroll_y, width, height, border_radius
                    ));
                }
                DisplayCommand::PopClip => {
                    json.push_str(r#"{"type":"popClip"}"#);
                }
                DisplayCommand::PushOpacity { opacity } => {
                    json.push_str(&format!(
                        r#"{{"type":"pushOpacity","opacity":{}}}"#,
                        opacity
                    ));
                }
                DisplayCommand::PopOpacity => {
                    json.push_str(r#"{"type":"popOpacity"}"#);
                }
                DisplayCommand::PushTransform { a, b, c, d, tx, ty } => {
                    json.push_str(&format!(
                        r#"{{"type":"pushTransform","a":{},"b":{},"c":{},"d":{},"tx":{},"ty":{}}}"#,
                        a, b, c, d, tx, ty
                    ));
                }
                DisplayCommand::PopTransform => {
                    json.push_str(r#"{"type":"popTransform"}"#);
                }
                DisplayCommand::DrawGradientRect { x1, y1, x2, y2, gradient, border_radius } => {
                    let escaped_grad = gradient.replace('\\', "\\\\").replace('"', "\\\"");
                    json.push_str(&format!(
                        r#"{{"type":"gradientRect","x1":{},"y1":{},"x2":{},"y2":{},"gradient":"{}","borderRadius":{}}}"#,
                        x1, y1 - scroll_y, x2, y2 - scroll_y, escaped_grad, border_radius
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

    /// Set theme preset and regenerate dynamic CSS variables
    pub fn set_theme_preset(&mut self, preset: crate::theme_engine::HeritagePreset) -> String {
        self.theme_engine.apply_preset(preset);
        self.is_dirty = true;
        self.theme_engine.generate_theme_css()
    }

    /// Extract reader mode article from currently loaded DOM
    pub fn extract_reader_mode(&self) -> Option<crate::reader_mode::ReaderArticle> {
        self.dom_root.as_ref().and_then(|dom| {
            crate::reader_mode::ReaderModeEngine::parse(dom, &self.current_title)
        })
    }

    pub fn toggle_reader_mode(&mut self, viewport_w: f32, viewport_h: f32) {
        if self.reader_mode_active {
            self.reader_mode_active = false;
            self.reader_article = None;
            if let Some(url) = self.current_url.as_ref().map(|u| u.as_string()) {
                let _ = self.load_url(&url, viewport_w, viewport_h);
            }
        } else if let Some(article) = self.extract_reader_mode() {
            self.reader_mode_active = true;
            let reader_html = format!(
                "<html><body style='margin:0 auto;max-width:680px;padding:40px 20px;font-family:Georgia,serif;background:#fefefe;color:#1a1a1a;line-height:1.8'>\
                 <h1 style='font-size:28px;margin-bottom:8px'>{}</h1>\
                 <p style='color:#666;font-size:14px;margin-bottom:24px'>{} min read</p>\
                 {}</body></html>",
                article.title, article.estimated_reading_time_mins, article.content_html
            );
            self.reader_article = Some(article);
            let _ = self.load_html(&reader_html, viewport_w, viewport_h);
        }
    }

    pub fn summarize_page(&mut self) -> String {
        if let Some(ref article) = self.reader_article {
            self.ai_assistant.summarize(&article.text_content, 3)
        } else if let Some(ref dom) = self.dom_root {
            if let Some(article) = crate::reader_mode::ReaderModeEngine::parse(dom, &self.current_title) {
                let summary = self.ai_assistant.summarize(&article.text_content, 3);
                self.page_summary = Some(summary.clone());
                summary
            } else {
                "No readable content found to summarize.".to_string()
            }
        } else {
            "No page loaded.".to_string()
        }
    }

    pub fn extract_keywords(&self) -> Vec<String> {
        if let Some(ref article) = self.reader_article {
            self.ai_assistant.extract_keywords(&article.text_content)
        } else {
            Vec::new()
        }
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

#[allow(dead_code)]
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

fn apply_node_animations(
    node: &NodePtr,
    elapsed_sec: f32,
    keyframes: &HashMap<String, KeyframeAnimation>,
    mutated: &mut bool,
) {
    {
        let mut b = node.borrow_mut();
        if let NodeType::Element { ref mut style, .. } = b.node_type {
            if let Some(anim_val) = style.get("animation").or_else(|| style.get("animation-name")).cloned() {
                let specs = parse_animation_shorthand(&anim_val);
                for spec in specs {
                    if let Some(kf) = keyframes.get(&spec.name) {
                        let duration = if spec.duration_sec > 0.0 { spec.duration_sec } else { 1.0 };
                        let active_time = (elapsed_sec - spec.delay_sec).max(0.0);
                        let cycle = active_time / duration;
                        if cycle < spec.iteration_count {
                            let progress_in_cycle = cycle.fract();
                            let effective_progress = if spec.direction == "reverse" {
                                1.0 - progress_in_cycle
                            } else if spec.direction == "alternate" {
                                if (cycle.floor() as u64) % 2 == 1 {
                                    1.0 - progress_in_cycle
                                } else {
                                    progress_in_cycle
                                }
                            } else {
                                progress_in_cycle
                            };
                            let solved_p = spec.timing_fn.solve(effective_progress);
                            let sampled = kf.sample(solved_p);
                            for (k, v) in sampled {
                                style.insert(k, v);
                            }
                            *mutated = true;
                        }
                    }
                }
            }
        }
    }

    let children = node.borrow().children.clone();
    for child in children {
        apply_node_animations(&child, elapsed_sec, keyframes, mutated);
    }
}

// ============================================================================
// MULTI-PROCESS SANDBOX ARCHITECTURE & IPC BUS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessKind {
    BrowserMain,
    RendererSandbox,
    NetworkProcess,
    GpuCompositor,
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub disallow_disk_io: bool,
    pub disallow_raw_sockets: bool,
    pub isolated_origin: Option<String>,
    pub max_memory_mb: usize,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        SandboxPolicy {
            disallow_disk_io: true,
            disallow_raw_sockets: true,
            isolated_origin: None,
            max_memory_mb: 512,
        }
    }
}

#[derive(Debug, Clone)]
pub enum IpcMessage {
    Navigate { url: String },
    RenderFrame { width: f32, height: f32 },
    EvalScript { script: String },
    CdpCommand { id: u64, method: String, params: String },
    CdpResponse { id: u64, result: String, error: Option<String> },
}

pub struct IpcBus {
    pub process_kind: ProcessKind,
    pub sandbox_policy: SandboxPolicy,
    message_queue: Vec<IpcMessage>,
}

impl IpcBus {
    pub fn new(kind: ProcessKind, policy: SandboxPolicy) -> Self {
        IpcBus {
            process_kind: kind,
            sandbox_policy: policy,
            message_queue: Vec::new(),
        }
    }

    pub fn send(&mut self, msg: IpcMessage) {
        self.message_queue.push(msg);
    }

    pub fn drain_messages(&mut self) -> Vec<IpcMessage> {
        self.message_queue.drain(..).collect()
    }
}

// ============================================================================
// CHROME DEVTOOLS PROTOCOL (CDP) INSPECTOR ENGINE
// ============================================================================

pub struct CdpInspector;

impl CdpInspector {
    pub fn handle_command(engine: &mut AxomaiEngine, id: u64, method: &str, params: &str) -> String {
        match method {
            "Page.navigate" => {
                let target_url = params.trim().trim_matches('"');
                let _ = engine.load_url(target_url, 1920.0, 1080.0);
                format!(r#"{{"id":{},"result":{{"frameId":"main","loaderId":"1"}}}}"#, id)
            }
            "DOM.getDocument" => {
                let title = &engine.current_title;
                let url = engine.current_url.as_ref().map(|u| u.as_string()).unwrap_or_default();
                format!(
                    "{{\"id\":{},\"result\":{{\"root\":{{\"nodeId\":1,\"nodeType\":9,\"nodeName\":\"#document\",\"documentURL\":\"{}\",\"title\":\"{}\"}}}}}}",
                    id, url, title
                )
            }
            "DOM.querySelector" => {
                let sel = params.trim().trim_matches('"');
                if let Some(ref root) = engine.dom_root {
                    if let Some(matched) = crate::html_parser::query_selector(root, sel) {
                        let node_id = matched.borrow().node_id;
                        return format!(r#"{{"id":{},"result":{{"nodeId":{}}}}}"#, id, node_id);
                    }
                }
                format!(r#"{{"id":{},"result":{{"nodeId":0}}}}"#, id)
            }
            "Runtime.evaluate" => {
                let script = params.trim().trim_matches('"');
                let _ = engine.js_engine.execute(script);
                format!(r#"{{"id":{},"result":{{"result":{{"type":"string","value":"evaluated"}}}}}}"#, id)
            }
            "Network.getResponseBody" => {
                let title = &engine.current_title;
                format!(r#"{{"id":{},"result":{{"body":"<html><title>{}</title></html>","base64Encoded":false}}}}"#, id, title)
            }
            _ => {
                format!(r#"{{"id":{},"error":{{"code":-32601,"message":"Method not found"}}}}"#, id)
            }
        }
    }
}




