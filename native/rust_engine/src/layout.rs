use crate::html_parser::{NodePtr, NodeType};
use std::collections::HashMap;

pub fn parse_px(val: &str, default_val: f32) -> f32 {
    let trimmed = val.trim().to_lowercase();
    if trimmed.is_empty() {
        return default_val;
    }
    if trimmed.ends_with("px") {
        if let Ok(num) = trimmed[..trimmed.len() - 2].parse::<f32>() {
            return num;
        }
    }
    trimmed.parse::<f32>().unwrap_or(default_val)
}

pub fn get_href(node: &NodePtr) -> String {
    let mut curr = Some(node.clone());
    while let Some(n) = curr {
        let b = n.borrow();
        if let NodeType::Element {
            ref tag,
            ref attributes,
            ..
        } = b.node_type
        {
            if tag == "a" {
                if let Some(href) = attributes.get("href") {
                    return href.clone();
                }
            }
        }
        curr = b.parent.as_ref().and_then(|w| w.upgrade());
    }
    String::new()
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoxType {
    Block,
    Inline,
    Text,
    Image,
    Input,
    Button,
}

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub box_type: BoxType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub children: Vec<LayoutBox>,
    pub word: String,
    pub font_size: f32,
    pub font_weight: String,
    pub font_style: String,
    pub font_family: String,
    pub style: HashMap<String, String>,
    pub href: String,
    pub image_bytes: Vec<u8>,
    pub value: String,
    pub placeholder: String,
    pub is_focused: bool,
}

impl LayoutBox {
    pub fn new(box_type: BoxType, style: HashMap<String, String>, href: String) -> Self {
        let font_size = parse_px(style.get("font-size").map(|s| s.as_str()).unwrap_or("16px"), 16.0);
        let font_weight = style.get("font-weight").cloned().unwrap_or_else(|| "normal".to_string());
        let font_style = style.get("font-style").cloned().unwrap_or_else(|| "normal".to_string());
        let font_family = style.get("font-family").cloned().unwrap_or_else(|| "sans-serif".to_string());

        LayoutBox {
            box_type,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            children: Vec::new(),
            word: String::new(),
            font_size,
            font_weight,
            font_style,
            font_family,
            style,
            href,
            image_bytes: Vec::new(),
            value: String::new(),
            placeholder: String::new(),
            is_focused: false,
        }
    }

    pub fn layout(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        self.x = x;
        self.y = y;

        if self.box_type == BoxType::Block {
            self.width = max_width;

            let margin_top = parse_px(self.style.get("margin-top").map(|s| s.as_str()).unwrap_or("0px"), 0.0);
            let margin_bottom = parse_px(self.style.get("margin-bottom").map(|s| s.as_str()).unwrap_or("0px"), 0.0);
            let padding_val = self.style.get("padding").map(|s| s.as_str()).unwrap_or("0px");
            let padding_top = parse_px(self.style.get("padding-top").map(|s| s.as_str()).unwrap_or(padding_val), 0.0);
            let padding_bottom = parse_px(self.style.get("padding-bottom").map(|s| s.as_str()).unwrap_or(padding_val), 0.0);
            let padding_left = parse_px(self.style.get("padding-left").map(|s| s.as_str()).unwrap_or(padding_val), 0.0);
            let padding_right = parse_px(self.style.get("padding-right").map(|s| s.as_str()).unwrap_or(padding_val), 0.0);

            let content_width = (self.width - padding_left - padding_right).max(0.0);
            let mut cursor_y = y + margin_top + padding_top;
            let child_x = x + padding_left;

            let mut line_boxes = Vec::new();
            let mut i = 0;

            while i < self.children.len() {
                if self.children[i].box_type == BoxType::Block {
                    if !line_boxes.is_empty() {
                        cursor_y += self.layout_inline_lines(&line_boxes, child_x, cursor_y, content_width);
                        line_boxes.clear();
                    }
                    let child_height = self.children[i].layout(child_x, cursor_y, content_width);
                    cursor_y += child_height;
                } else {
                    line_boxes.push(i);
                }
                i += 1;
            }

            if !line_boxes.is_empty() {
                cursor_y += self.layout_inline_lines(&line_boxes, child_x, cursor_y, content_width);
            }

            self.height = (cursor_y - y) + margin_bottom + padding_bottom;
            return self.height;
        }

        0.0
    }

    fn layout_inline_lines(
        &mut self,
        inline_indices: &[usize],
        x: f32,
        y: f32,
        max_width: f32,
    ) -> f32 {
        let mut cursor_x = x;
        let mut cursor_y = y;
        let mut line_height: f32 = 20.0;

        for &idx in inline_indices {
            let child = &mut self.children[idx];

            if child.box_type == BoxType::Text {
                let char_w = child.font_size * 0.6;
                line_height = line_height.max(child.font_size * 1.3);

                let words: Vec<&str> = child.word.split(' ').collect();
                let n_words = words.len();

                for (w_idx, word) in words.iter().enumerate() {
                    let space = if w_idx < n_words - 1 { " " } else { "" };
                    let word_str = format!("{}{}", word, space);
                    let w = word_str.len() as f32 * char_w;

                    if cursor_x + w > x + max_width && cursor_x > x {
                        cursor_x = x;
                        cursor_y += line_height;
                    }

                    let mut word_box = LayoutBox::new(BoxType::Text, child.style.clone(), child.href.clone());
                    word_box.word = word_str;
                    word_box.x = cursor_x;
                    word_box.y = cursor_y;
                    word_box.width = w;
                    word_box.height = line_height;
                    word_box.font_size = child.font_size;
                    word_box.font_weight = child.font_weight.clone();
                    word_box.font_style = child.font_style.clone();
                    word_box.font_family = child.font_family.clone();

                    child.children.push(word_box);
                    cursor_x += w;
                }
            }
        }

        (cursor_y - y) + line_height
    }
}

pub fn build_layout_tree(node: &NodePtr, current_url: Option<&crate::network::URL>) -> Option<LayoutBox> {
    let node_borrow = node.borrow();

    match node_borrow.node_type {
        NodeType::Element {
            ref tag,
            ref attributes,
            ref style,
        } => {
            let display = style.get("display").map(|s| s.as_str()).unwrap_or("block");
            if display == "none" {
                return None;
            }

            let href = get_href(node);

            if tag == "img" {
                let mut img_box = LayoutBox::new(BoxType::Image, style.clone(), href);
                let src = attributes.get("src").cloned().unwrap_or_default();

                if !src.is_empty() {
                    if let Some(url_obj) = current_url {
                        let full_src = url_obj.resolve(&src);
                        if let Ok(img_url) = crate::network::URL::parse(&full_src) {
                            let (_, bytes) = img_url.request_bytes();
                            img_box.image_bytes = bytes;
                        }
                    }
                }

                let w_str = attributes.get("width").or_else(|| style.get("width")).map(|s| s.as_str()).unwrap_or("200px");
                let h_str = attributes.get("height").or_else(|| style.get("height")).map(|s| s.as_str()).unwrap_or("150px");

                img_box.width = parse_px(w_str, 200.0);
                img_box.height = parse_px(h_str, 150.0);
                return Some(img_box);
            }

            if tag == "input" {
                let mut input_box = LayoutBox::new(BoxType::Input, style.clone(), href);
                input_box.value = attributes.get("value").cloned().unwrap_or_default();
                input_box.placeholder = attributes.get("placeholder").cloned().unwrap_or_default();

                let w_str = attributes.get("width").or_else(|| style.get("width")).map(|s| s.as_str()).unwrap_or("180px");
                let h_str = attributes.get("height").or_else(|| style.get("height")).map(|s| s.as_str()).unwrap_or("30px");

                input_box.width = parse_px(w_str, 180.0);
                input_box.height = parse_px(h_str, 30.0);
                return Some(input_box);
            }

            if tag == "button" {
                let mut btn_box = LayoutBox::new(BoxType::Button, style.clone(), href);
                let w_str = attributes.get("width").or_else(|| style.get("width")).map(|s| s.as_str()).unwrap_or("100px");
                let h_str = attributes.get("height").or_else(|| style.get("height")).map(|s| s.as_str()).unwrap_or("32px");

                btn_box.width = parse_px(w_str, 100.0);
                btn_box.height = parse_px(h_str, 32.0);

                for child in &node_borrow.children {
                    if let Some(child_box) = build_layout_tree(child, current_url) {
                        btn_box.children.push(child_box);
                    }
                }
                return Some(btn_box);
            }

            let box_type = if display == "block" || display == "flex" || display == "table" {
                BoxType::Block
            } else {
                BoxType::Inline
            };

            let mut root_box = LayoutBox::new(box_type, style.clone(), href);
            for child in &node_borrow.children {
                if let Some(child_box) = build_layout_tree(child, current_url) {
                    root_box.children.push(child_box);
                }
            }
            Some(root_box)
        }

        NodeType::Text { ref text } => {
            let cleaned = text.replace('\n', " ").trim().to_string();
            if cleaned.is_empty() {
                return None;
            }

            let parent_style = if let Some(ref parent_weak) = node_borrow.parent {
                if let Some(parent_rc) = parent_weak.upgrade() {
                    let pb = parent_rc.borrow();
                    if let NodeType::Element { ref style, .. } = pb.node_type {
                        style.clone()
                    } else {
                        HashMap::new()
                    }
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            };

            let href = get_href(node);
            let mut text_box = LayoutBox::new(BoxType::Text, parent_style, href);
            text_box.word = cleaned;
            Some(text_box)
        }
    }
}
