use crate::css_parser::{style_tree, CSSParser, DEFAULT_UA_STYLES};
use crate::html_parser::{HTMLParser, NodeData, NodePtr, NodeType};
use crate::js_engine::RustJSEngine;
use crate::layout::{build_layout_tree, LayoutBox};
use crate::network::URL;
use crate::painter::{build_display_list, DisplayCommand};

pub struct AxomaiEngine {
    pub current_url: Option<URL>,
    pub dom_root: Option<NodePtr>,
    pub layout_root: Option<LayoutBox>,
    pub display_list: Vec<DisplayCommand>,
    pub max_scroll_y: f32,
    pub focused_input_idx: Option<usize>,
    pub status_message: String,
}

impl AxomaiEngine {
    pub fn new() -> Self {
        AxomaiEngine {
            current_url: None,
            dom_root: None,
            layout_root: None,
            display_list: Vec::new(),
            max_scroll_y: 0.0,
            focused_input_idx: None,
            status_message: "Ready".to_string(),
        }
    }

    pub fn load_url(&mut self, url_str: &str, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        let url = URL::parse(url_str)?;
        self.current_url = Some(url.clone());

        let (_headers, body) = url.request();
        self.load_html(&body, viewport_w, viewport_h)
    }

    pub fn load_html(&mut self, html_content: &str, viewport_w: f32, viewport_h: f32) -> Result<(), String> {
        // 1. HTML DOM Parse
        let dom_root = HTMLParser::new(html_content).parse();

        // 2. Extract & Execute <script>
        let mut scripts = Vec::new();
        extract_script_tags(&dom_root, &mut scripts);
        if !scripts.is_empty() {
            let mut js_engine = RustJSEngine::new();
            for js in scripts {
                js_engine.execute(&js, Some(&dom_root));
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

        style_tree(&dom_root, &all_rules);

        self.dom_root = Some(dom_root.clone());

        // 4. Layout
        if let Some(mut layout_box) = build_layout_tree(&dom_root, self.current_url.as_ref()) {
            let total_h = layout_box.layout(0.0, 0.0, viewport_w.max(800.0));
            self.max_scroll_y = (total_h - viewport_h).max(0.0);

            // 5. Display List
            let mut list = Vec::new();
            build_display_list(&layout_box, &mut list);

            self.layout_root = Some(layout_box);
            self.display_list = list;
        } else {
            self.layout_root = None;
            self.display_list.clear();
            self.max_scroll_y = 0.0;
        }

        self.status_message = "Page Loaded Successfully".to_string();
        Ok(())
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
                        return None;
                    }
                }

                DisplayCommand::DrawButton {
                    x, y, width, height, ..
                } => {
                    if click_x >= *x && click_x <= *x + *width && abs_y >= *y && abs_y <= *y + *height {
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

fn extract_script_tags(node: &NodePtr, script_list: &mut Vec<String>) {
    let b = node.borrow();
    if let NodeType::Element { ref tag, .. } = b.node_type {
        if tag == "script" {
            for child in &b.children {
                let cb = child.borrow();
                if let NodeType::Text { ref text } = cb.node_type {
                    script_list.push(text.clone());
                }
            }
        }
    }
    for child in &b.children {
        extract_script_tags(child, script_list);
    }
}
