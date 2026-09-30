use crate::css_parser::{style_tree, CSSParser, Rule, DEFAULT_UA_STYLES};
use crate::html_parser::{HTMLParser, NodePtr, NodeType};
use crate::js_engine::V8JSEngine;
use crate::layout::{build_layout_tree, LayoutBox};
use crate::network::URL;
use crate::painter::{build_display_list, DisplayCommand};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

static NEXT_ENGINE_ID: AtomicUsize = AtomicUsize::new(1);

pub struct NavigationResponse {
    pub engine_id: usize,
    pub url: URL,
    pub body: String,
    pub error: Option<String>,
}

static NAVIGATION_QUEUE: Mutex<Option<Vec<NavigationResponse>>> = Mutex::new(None);
static HAS_PENDING_NAVIGATION: AtomicBool = AtomicBool::new(false);

fn push_navigation_response(resp: NavigationResponse) {
    let mut lock = NAVIGATION_QUEUE.lock().unwrap();
    if lock.is_none() {
        *lock = Some(Vec::new());
    }
    if let Some(ref mut q) = *lock {
        q.push(resp);
    }
    HAS_PENDING_NAVIGATION.store(true, Ordering::SeqCst);
}

fn drain_navigation_responses_for_engine(engine_id: usize) -> Vec<NavigationResponse> {
    let mut lock = NAVIGATION_QUEUE.lock().unwrap();
    if let Some(ref mut q) = *lock {
        let (matching, remaining): (Vec<_>, Vec<_>) = q.drain(..).partition(|r| r.engine_id == engine_id);
        *q = remaining;
        if q.is_empty() {
            HAS_PENDING_NAVIGATION.store(false, Ordering::SeqCst);
        }
        matching
    } else {
        Vec::new()
    }
}

pub struct AxomaiEngine {
    pub engine_id: usize,
    pub current_url: Option<URL>,
    pub dom_root: Option<NodePtr>,
    pub layout_root: Option<LayoutBox>,
    pub display_list: Vec<DisplayCommand>,
    pub max_scroll_y: f32,
    pub focused_input_idx: Option<usize>,
    pub status_message: String,
    pub js_engine: V8JSEngine,
    pub active_css_rules: Vec<Rule>,
}

impl AxomaiEngine {
    pub fn new() -> Self {
        let engine_id = NEXT_ENGINE_ID.fetch_add(1, Ordering::SeqCst);
        let mut js_engine = V8JSEngine::new();
        js_engine.engine_id = engine_id;

        AxomaiEngine {
            engine_id,
            current_url: None,
            dom_root: None,
            layout_root: None,
            display_list: Vec::new(),
            max_scroll_y: 0.0,
            focused_input_idx: None,
            status_message: "Ready".to_string(),
            js_engine,
            active_css_rules: Vec::new(),
        }
    }

    /// Load URL with non-blocking asynchronous network fetching for http/https
    pub fn load_url(&mut self, url_str: &str, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        let url = URL::parse(url_str)?;
        self.current_url = Some(url.clone());

        // Local or inline schemes load synchronously
        if url.scheme == "data" || url.scheme == "file" || url_str.starts_with("about:") {
            let (_headers, body) = url.request();
            return self.load_html(&body, viewport_w, viewport_h);
        }

        // Network schemes (http/https): execute non-blocking in background worker thread with engine_id
        self.status_message = format!("Connecting to {}...", url.host);
        let url_clone = url.clone();
        let engine_id = self.engine_id;
        thread::spawn(move || {
            let (_headers, body) = url_clone.request();
            push_navigation_response(NavigationResponse {
                engine_id,
                url: url_clone,
                body,
                error: None,
            });
        });

        Ok(())
    }

    pub fn load_html(&mut self, html_content: &str, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        // 1. HTML DOM Parse
        let dom_root = HTMLParser::new(html_content).parse();

        // 2. Extract & Execute <script> via persistent V8 Engine Context
        let url_str = self
            .current_url
            .as_ref()
            .map(|u| u.raw.as_str())
            .unwrap_or("about:blank");

        // Establish ONE persistent V8 Context for the page
        self.js_engine.reset_page_context(Some(&dom_root), url_str);

        let mut scripts = Vec::new();
        extract_script_tags(&dom_root, &mut scripts);
        for entry in scripts {
            match entry {
                ScriptEntry::Inline(code) => {
                    let _ = self.js_engine.execute(&code);
                }
                ScriptEntry::External(src) => {
                    let resolved_url = if let Some(ref current) = self.current_url {
                        current.resolve(&src)
                    } else {
                        src.clone()
                    };
                    println!("[Axomai Engine] Fetching external script: {}", resolved_url);
                    if let Ok(url_obj) = URL::parse(&resolved_url) {
                        if url_obj.scheme == "file" || url_obj.scheme == "data" {
                            let (_headers, js_code) = url_obj.request();
                            if !js_code.is_empty() {
                                let _ = self.js_engine.execute(&js_code);
                            }
                        } else {
                            // Non-hanging script download: timeout-bounded network fetch
                            let handle = thread::spawn(move || {
                                match ureq::get(&resolved_url).timeout(Duration::from_secs(3)).call() {
                                    Ok(resp) => resp.into_string().unwrap_or_default(),
                                    Err(_) => String::new(),
                                }
                            });
                            if let Ok(js_code) = handle.join() {
                                if !js_code.is_empty() {
                                    let _ = self.js_engine.execute(&js_code);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Extract <style> & Style Tree
        let mut author_css_list = Vec::new();
        extract_style_tags(&dom_root, &mut author_css_list);
        let author_css = author_css_list.join("\n");

        let ua_rules = CSSParser::new(DEFAULT_UA_STYLES).parse();
        let author_rules = CSSParser::new(&author_css).parse();
        let mut all_rules = ua_rules;
        all_rules.extend(author_rules);

        self.active_css_rules = all_rules;
        self.dom_root = Some(dom_root);

        // 4. Restyle, Layout & Display List
        self.restyle_and_relayout(viewport_w, viewport_h);

        self.status_message = "Page Loaded Successfully".to_string();
        Ok(())
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

                // 3. Rebuild Display List
                let mut list = Vec::new();
                build_display_list(&layout_box, &mut list);

                self.layout_root = Some(layout_box);
                self.display_list = list;
            } else {
                self.layout_root = None;
                self.display_list.clear();
                self.max_scroll_y = 0.0;
            }
        }
    }

    pub fn handle_click(&mut self, click_x: f32, click_y: f32, scroll_y: f32) -> Option<String> {
        let abs_y = click_y + scroll_y;
        let query = self.get_focused_input_val();

        // Unfocus previous input
        if let Some(idx) = self.focused_input_idx {
            if idx < self.display_list.len() {
                if let DisplayCommand::DrawInput { ref mut is_focused, .. } = self.display_list[idx] {
                    *is_focused = false;
                }
            }
            self.focused_input_idx = None;
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
                    } else if key_str == "Return" || key_str == "\r" || key_str == "\n" {
                        let query = value.trim().to_string();
                        if !query.is_empty() {
                            return Some(format!("https://html.duckduckgo.com/html/?q={}", query));
                        }
                    } else if key_str.len() == 1 {
                        value.push_str(key_str);
                    }
                }
            }
        }
        None
    }

    pub fn handle_hover(&self, hover_x: f32, hover_y: f32, scroll_y: f32) -> Option<String> {
        let abs_y = hover_y + scroll_y;
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

        // 1. Process completed non-blocking page navigations for THIS engine instance
        let completed_navs = drain_navigation_responses_for_engine(self.engine_id);
        for nav in completed_navs {
            self.current_url = Some(nav.url);
            let _ = self.load_html(&nav.body, viewport_w, viewport_h);
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

        executed
    }

    /// Serialize current display list commands to JSON for desktop WebView/canvas rendering bridge
    pub fn get_display_list_json(&self) -> String {
        let mut json = String::from("[");
        for (i, cmd) in self.display_list.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            match cmd {
                DisplayCommand::DrawRect { x1, y1, x2, y2, color } => {
                    json.push_str(&format!(
                        r#"{{"type":"rect","x1":{},"y1":{},"x2":{},"y2":{},"color":"{}"}}"#,
                        x1, y1, x2, y2, color
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
                        x, y, width, height, escaped_text, font_size, font_weight, font_style, color, href
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
                        x, y, width, height, value, placeholder, is_focused
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
                        x, y, width, height, label
                    ));
                }
                DisplayCommand::DrawImage {
                    x, y, width, height, ..
                } => {
                    json.push_str(&format!(
                        r#"{{"type":"image","x":{},"y":{},"width":{},"height":{}}}"#,
                        x, y, width, height
                    ));
                }
            }
        }
        json.push(']');
        json
    }

    /// Check if there are pending async fetch responses, background navigations, or timers ready for event loop processing
    pub fn has_pending_events(&self) -> bool {
        HAS_PENDING_NAVIGATION.load(Ordering::SeqCst) || self.js_engine.has_pending_events()
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
    Inline(String),
    External(String),
}

fn extract_script_tags(node: &NodePtr, script_list: &mut Vec<ScriptEntry>) {
    let b = node.borrow();
    if let NodeType::Element { ref tag, ref attributes, .. } = b.node_type {
        if tag == "script" {
            if let Some(src) = attributes.get("src") {
                if !src.trim().is_empty() {
                    script_list.push(ScriptEntry::External(src.trim().to_string()));
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
                script_list.push(ScriptEntry::Inline(inline_code));
            }
        }
    }
    for child in &b.children {
        extract_script_tags(child, script_list);
    }
}

