use crate::html_parser::{NodePtr, NodeType};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    pub fn padding_box(&self) -> Rect {
        Rect {
            x: self.content.x - self.padding.left,
            y: self.content.y - self.padding.top,
            width: self.content.width + self.padding.left + self.padding.right,
            height: self.content.height + self.padding.top + self.padding.bottom,
        }
    }

    pub fn border_box(&self) -> Rect {
        let p = self.padding_box();
        Rect {
            x: p.x - self.border.left,
            y: p.y - self.border.top,
            width: p.width + self.border.left + self.border.right,
            height: p.height + self.border.top + self.border.bottom,
        }
    }

    pub fn margin_box(&self) -> Rect {
        let b = self.border_box();
        Rect {
            x: b.x - self.margin.left,
            y: b.y - self.margin.top,
            width: b.width + self.margin.left + self.margin.right,
            height: b.height + self.margin.top + self.margin.bottom,
        }
    }
}

pub fn parse_px(val: &str, default_val: f32) -> f32 {
    parse_length(val, 0.0, default_val)
}

pub fn parse_length(val: &str, containing_len: f32, default_val: f32) -> f32 {
    let trimmed = val.trim().to_lowercase();
    if trimmed.is_empty() || trimmed == "auto" {
        return default_val;
    }
    if trimmed.ends_with("px") {
        if let Ok(num) = trimmed[..trimmed.len() - 2].trim().parse::<f32>() {
            return num;
        }
    } else if trimmed.ends_with('%') {
        if let Ok(pct) = trimmed[..trimmed.len() - 1].trim().parse::<f32>() {
            return (pct / 100.0) * containing_len;
        }
    } else if trimmed.ends_with("em") || trimmed.ends_with("rem") {
        let unit_len = if trimmed.ends_with("rem") { 3 } else { 2 };
        if let Ok(mult) = trimmed[..trimmed.len() - unit_len].trim().parse::<f32>() {
            return mult * 16.0;
        }
    } else if let Ok(num) = trimmed.parse::<f32>() {
        return num;
    }
    default_val
}

pub fn parse_edges(style: &HashMap<String, String>, prefix: &str, containing_width: f32) -> EdgeSizes {
    let base_val = style.get(prefix).map(|s| s.as_str()).unwrap_or("0px");
    let parts: Vec<&str> = base_val.split_whitespace().collect();

    let (mut top, mut right, mut bottom, mut left) = match parts.len() {
        1 => {
            let v = parse_length(parts[0], containing_width, 0.0);
            (v, v, v, v)
        }
        2 => {
            let v_tb = parse_length(parts[0], containing_width, 0.0);
            let v_lr = parse_length(parts[1], containing_width, 0.0);
            (v_tb, v_lr, v_tb, v_lr)
        }
        3 => {
            let v_t = parse_length(parts[0], containing_width, 0.0);
            let v_lr = parse_length(parts[1], containing_width, 0.0);
            let v_b = parse_length(parts[2], containing_width, 0.0);
            (v_t, v_lr, v_b, v_lr)
        }
        4 => (
            parse_length(parts[0], containing_width, 0.0),
            parse_length(parts[1], containing_width, 0.0),
            parse_length(parts[2], containing_width, 0.0),
            parse_length(parts[3], containing_width, 0.0),
        ),
        _ => (0.0, 0.0, 0.0, 0.0),
    };

    if let Some(v) = style.get(&format!("{}-top", prefix)) {
        top = parse_length(v, containing_width, top);
    }
    if let Some(v) = style.get(&format!("{}-right", prefix)) {
        right = parse_length(v, containing_width, right);
    }
    if let Some(v) = style.get(&format!("{}-bottom", prefix)) {
        bottom = parse_length(v, containing_width, bottom);
    }
    if let Some(v) = style.get(&format!("{}-left", prefix)) {
        left = parse_length(v, containing_width, left);
    }

    EdgeSizes {
        top,
        right,
        bottom,
        left,
    }
}

pub fn parse_border_properties(style: &mut HashMap<String, String>, containing_width: f32) -> EdgeSizes {
    let mut border_widths = EdgeSizes::default();
    let mut resolved_color = style.get("border-color").cloned();

    // 1. Check border shorthand: e.g. "1px solid red" or "2px #333 solid"
    if let Some(border_shorthand) = style.get("border").cloned() {
        let parts: Vec<&str> = border_shorthand.split_whitespace().collect();
        for p in &parts {
            if p.ends_with("px") || p.ends_with('%') || p.ends_with("em") || p.ends_with("rem") || p.parse::<f32>().is_ok() {
                let w = parse_length(p, containing_width, 0.0);
                border_widths = EdgeSizes { top: w, right: w, bottom: w, left: w };
            } else if *p != "solid" && *p != "dashed" && *p != "dotted" && *p != "none" && *p != "hidden" && *p != "double" {
                if resolved_color.is_none() {
                    resolved_color = Some(p.to_string());
                }
            }
        }
    }

    // 2. Check individual side shorthands: border-top, border-right, border-bottom, border-left
    for (prop, side) in &[
        ("border-top", "top"),
        ("border-right", "right"),
        ("border-bottom", "bottom"),
        ("border-left", "left"),
    ] {
        if let Some(val) = style.get(*prop) {
            for p in val.split_whitespace() {
                if p.ends_with("px") || p.ends_with('%') || p.ends_with("em") || p.ends_with("rem") || p.parse::<f32>().is_ok() {
                    let w = parse_length(p, containing_width, 0.0);
                    match *side {
                        "top" => border_widths.top = w,
                        "right" => border_widths.right = w,
                        "bottom" => border_widths.bottom = w,
                        "left" => border_widths.left = w,
                        _ => {}
                    }
                } else if p != "solid" && p != "dashed" && p != "dotted" && p != "none" && p != "hidden" {
                    if resolved_color.is_none() {
                        resolved_color = Some(p.to_string());
                    }
                }
            }
        }
    }

    // 3. Check explicit border-width
    let explicit_widths = parse_edges(style, "border-width", containing_width);
    if explicit_widths.top > 0.0 || explicit_widths.right > 0.0 || explicit_widths.bottom > 0.0 || explicit_widths.left > 0.0 {
        border_widths = explicit_widths;
    }

    if let Some(ref c) = resolved_color {
        style.entry("border-color".to_string()).or_insert_with(|| c.clone());
    }

    border_widths
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
    Flex,
    Text,
    Image,
    Input,
    Button,
    AnonymousBlock,
}

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub box_type: BoxType,
    pub dimensions: Dimensions,
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
            dimensions: Dimensions::default(),
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
        self.dimensions.padding = parse_edges(&self.style, "padding", max_width);
        self.dimensions.margin = parse_edges(&self.style, "margin", max_width);
        self.dimensions.border = parse_border_properties(&mut self.style, max_width);

        match self.box_type {
            BoxType::Flex => self.layout_flex(x, y, max_width),
            BoxType::Block | BoxType::AnonymousBlock => self.layout_block(x, y, max_width),
            BoxType::Image | BoxType::Input | BoxType::Button => self.layout_leaf(x, y),
            _ => self.layout_block(x, y, max_width),
        }
    }

    fn layout_block(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        let explicit_w = self.style.get("width").map(|w| parse_length(w, max_width, -1.0)).unwrap_or(-1.0);

        let content_width = if explicit_w >= 0.0 {
            explicit_w
        } else {
            (max_width
                - self.dimensions.margin.left
                - self.dimensions.margin.right
                - self.dimensions.padding.left
                - self.dimensions.padding.right
                - self.dimensions.border.left
                - self.dimensions.border.right)
                .max(0.0)
        };

        let is_margin_left_auto = self.style.get("margin-left").map(|s| s.trim() == "auto").unwrap_or(false);
        let is_margin_right_auto = self.style.get("margin-right").map(|s| s.trim() == "auto").unwrap_or(false);

        if is_margin_left_auto && is_margin_right_auto && explicit_w >= 0.0 {
            let total_box_w = content_width + self.dimensions.padding.left + self.dimensions.padding.right + self.dimensions.border.left + self.dimensions.border.right;
            let auto_margin = ((max_width - total_box_w) / 2.0).max(0.0);
            self.dimensions.margin.left = auto_margin;
            self.dimensions.margin.right = auto_margin;
        }

        self.dimensions.content.x = x + self.dimensions.margin.left + self.dimensions.border.left + self.dimensions.padding.left;
        self.dimensions.content.y = y + self.dimensions.margin.top + self.dimensions.border.top + self.dimensions.padding.top;
        self.dimensions.content.width = content_width;

        let child_x = self.dimensions.content.x;
        let mut cursor_y = self.dimensions.content.y;

        let mut line_boxes = Vec::new();
        let mut i = 0;

        while i < self.children.len() {
            let is_block_child = matches!(
                self.children[i].box_type,
                BoxType::Block | BoxType::Flex | BoxType::AnonymousBlock
            );

            if is_block_child {
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

        let computed_content_height = (cursor_y - self.dimensions.content.y).max(0.0);
        let explicit_h = self.style.get("height").map(|h| parse_length(h, 0.0, -1.0)).unwrap_or(-1.0);
        self.dimensions.content.height = if explicit_h >= 0.0 { explicit_h } else { computed_content_height };

        let border_box = self.dimensions.border_box();
        self.x = border_box.x;
        self.y = border_box.y;
        self.width = border_box.width;
        self.height = border_box.height;

        self.dimensions.margin_box().height
    }

    fn layout_flex(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        let explicit_w = self.style.get("width").map(|w| parse_length(w, max_width, -1.0)).unwrap_or(-1.0);
        let content_width = if explicit_w >= 0.0 {
            explicit_w
        } else {
            (max_width - self.dimensions.margin.left - self.dimensions.margin.right - self.dimensions.padding.left - self.dimensions.padding.right - self.dimensions.border.left - self.dimensions.border.right).max(0.0)
        };

        self.dimensions.content.x = x + self.dimensions.margin.left + self.dimensions.border.left + self.dimensions.padding.left;
        self.dimensions.content.y = y + self.dimensions.margin.top + self.dimensions.border.top + self.dimensions.padding.top;
        self.dimensions.content.width = content_width;

        let flex_dir = self.style.get("flex-direction").map(|s| s.as_str()).unwrap_or("row");
        let justify = self.style.get("justify-content").map(|s| s.as_str()).unwrap_or("flex-start");
        let align_items = self.style.get("align-items").map(|s| s.as_str()).unwrap_or("stretch");
        let gap = parse_px(self.style.get("gap").map(|s| s.as_str()).unwrap_or("0px"), 0.0);

        let is_row = flex_dir == "row" || flex_dir == "row-reverse";
        let is_reverse = flex_dir == "row-reverse" || flex_dir == "column-reverse";

        let num_children = self.children.len();
        let mut child_sizes: Vec<(f32, f32)> = Vec::new();

        let explicit_h = self.style.get("height").map(|h| parse_length(h, 0.0, -1.0)).unwrap_or(-1.0);

        // 1. Initial measurement pass
        let default_child_w = if is_row {
            if num_children > 0 {
                let total_gap = gap * (num_children.saturating_sub(1) as f32);
                ((content_width - total_gap) / num_children as f32).max(0.0)
            } else {
                content_width
            }
        } else {
            content_width
        };

        for child in &mut self.children {
            let child_explicit_w = child.style.get("width").map(|w| parse_length(w, content_width, -1.0)).unwrap_or(-1.0);
            let target_w = if child_explicit_w >= 0.0 { child_explicit_w } else { default_child_w };

            let ch_h = child.layout(self.dimensions.content.x, self.dimensions.content.y, target_w);
            let ch_w = if child.width > 0.0 { child.width } else { target_w };
            child_sizes.push((ch_w, ch_h));
        }

        let mut max_cross_size: f32 = 0.0;
        let mut total_main_size: f32 = 0.0;

        for &(cw, ch) in &child_sizes {
            if is_row {
                total_main_size += cw;
                max_cross_size = max_cross_size.max(ch);
            } else {
                total_main_size += ch;
                max_cross_size = max_cross_size.max(cw);
            }
        }
        if num_children > 1 {
            total_main_size += gap * (num_children - 1) as f32;
        }

        let container_main_size = if is_row {
            content_width
        } else if explicit_h >= 0.0 {
            explicit_h
        } else {
            total_main_size
        };

        let main_free_space = (container_main_size - total_main_size).max(0.0);

        let (start_offset, item_spacing) = match justify {
            "center" => (main_free_space / 2.0, gap),
            "flex-end" => (main_free_space, gap),
            "space-between" => {
                let space = if num_children > 1 { main_free_space / (num_children - 1) as f32 } else { 0.0 };
                (0.0, gap + space)
            }
            "space-around" => {
                let space = if num_children > 0 { main_free_space / num_children as f32 } else { 0.0 };
                (space / 2.0, gap + space)
            }
            _ => (0.0, gap),
        };

        let indices: Vec<usize> = if is_reverse {
            (0..num_children).rev().collect()
        } else {
            (0..num_children).collect()
        };

        let mut cur_main = if is_row {
            self.dimensions.content.x + start_offset
        } else {
            self.dimensions.content.y + start_offset
        };

        for &idx in &indices {
            let (cw, ch) = child_sizes[idx];
            let child = &mut self.children[idx];

            if is_row {
                child.x = cur_main;
                child.dimensions.content.x = cur_main;

                let cross_y = match align_items {
                    "center" => self.dimensions.content.y + (max_cross_size - ch) / 2.0,
                    "flex-end" => self.dimensions.content.y + (max_cross_size - ch),
                    "stretch" => {
                        let has_explicit_h = child.style.contains_key("height");
                        if !has_explicit_h {
                            child.height = max_cross_size;
                            child.dimensions.content.height = max_cross_size;
                        }
                        self.dimensions.content.y
                    }
                    _ => self.dimensions.content.y,
                };
                child.y = cross_y;
                child.dimensions.content.y = cross_y;

                cur_main += cw + item_spacing;
            } else {
                child.y = cur_main;
                child.dimensions.content.y = cur_main;

                let cross_x = match align_items {
                    "center" => self.dimensions.content.x + (content_width - cw) / 2.0,
                    "flex-end" => self.dimensions.content.x + (content_width - cw),
                    "stretch" => {
                        let has_explicit_w = child.style.contains_key("width");
                        if !has_explicit_w {
                            child.width = content_width;
                            child.dimensions.content.width = content_width;
                        }
                        self.dimensions.content.x
                    }
                    _ => self.dimensions.content.x,
                };
                child.x = cross_x;
                child.dimensions.content.x = cross_x;

                cur_main += ch + item_spacing;
            }
        }

        self.dimensions.content.height = if explicit_h >= 0.0 {
            explicit_h
        } else if is_row {
            max_cross_size
        } else {
            total_main_size
        };

        let border_box = self.dimensions.border_box();
        self.x = border_box.x;
        self.y = border_box.y;
        self.width = border_box.width;
        self.height = border_box.height;

        self.dimensions.margin_box().height
    }

    fn layout_leaf(&mut self, x: f32, y: f32) -> f32 {
        self.dimensions.content.x = x + self.dimensions.margin.left + self.dimensions.border.left + self.dimensions.padding.left;
        self.dimensions.content.y = y + self.dimensions.margin.top + self.dimensions.border.top + self.dimensions.padding.top;
        self.dimensions.content.width = self.width;
        self.dimensions.content.height = self.height;

        let border_box = self.dimensions.border_box();
        self.x = border_box.x;
        self.y = border_box.y;
        self.width = border_box.width;
        self.height = border_box.height;

        self.dimensions.margin_box().height
    }

    fn layout_inline_lines(
        &mut self,
        inline_indices: &[usize],
        x: f32,
        y: f32,
        max_width: f32,
    ) -> f32 {
        let text_align = self.style.get("text-align").map(|s| s.as_str()).unwrap_or("left");

        // Clear previous word_boxes on text children to prevent reflow duplication
        for &idx in inline_indices {
            if self.children[idx].box_type == BoxType::Text {
                self.children[idx].children.clear();
            }
        }

        struct InlineItem {
            child_idx: usize,
            word_str: String,
            width: f32,
        }

        let mut lines: Vec<Vec<InlineItem>> = Vec::new();
        let mut current_line: Vec<InlineItem> = Vec::new();
        let mut current_line_w: f32 = 0.0;
        let mut line_height: f32 = 20.0;

        for &idx in inline_indices {
            let child = &self.children[idx];

            if child.box_type == BoxType::Text {
                let char_w = child.font_size * 0.6;
                line_height = line_height.max(child.font_size * 1.35);

                let words: Vec<&str> = child.word.split_whitespace().collect();
                let n_words = words.len();

                for (w_idx, word) in words.iter().enumerate() {
                    let space = if w_idx < n_words - 1 { " " } else { "" };
                    let word_str = format!("{}{}", word, space);
                    let w = word_str.len() as f32 * char_w;

                    if current_line_w + w > max_width && !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = Vec::new();
                        current_line_w = 0.0;
                    }

                    current_line_w += w;
                    current_line.push(InlineItem {
                        child_idx: idx,
                        word_str,
                        width: w,
                    });
                }
            } else {
                let w = if child.width > 0.0 { child.width } else { 50.0 };
                let h = if child.height > 0.0 { child.height } else { 20.0 };
                line_height = line_height.max(h);

                if current_line_w + w > max_width && !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_line_w = 0.0;
                }

                current_line_w += w;
                current_line.push(InlineItem {
                    child_idx: idx,
                    word_str: String::new(),
                    width: w,
                });
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        let mut cursor_y = y;

        for line in lines {
            let line_w: f32 = line.iter().map(|item| item.width).sum();
            let start_x = match text_align {
                "center" => x + ((max_width - line_w) / 2.0).max(0.0),
                "right" => x + (max_width - line_w).max(0.0),
                _ => x,
            };

            let mut cursor_x = start_x;

            for item in line {
                let child = &mut self.children[item.child_idx];
                if child.box_type == BoxType::Text {
                    let mut word_box = LayoutBox::new(BoxType::Text, child.style.clone(), child.href.clone());
                    word_box.word = item.word_str;
                    word_box.x = cursor_x;
                    word_box.y = cursor_y;
                    word_box.width = item.width;
                    word_box.height = line_height;
                    word_box.font_size = child.font_size;
                    word_box.font_weight = child.font_weight.clone();
                    word_box.font_style = child.font_style.clone();
                    word_box.font_family = child.font_family.clone();

                    child.children.push(word_box);
                } else {
                    child.x = cursor_x;
                    child.y = cursor_y;
                }
                cursor_x += item.width;
            }
            cursor_y += line_height;
        }

        (cursor_y - y).max(line_height)
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

            let box_type = if display == "flex" {
                BoxType::Flex
            } else if display == "block" || display == "table" {
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
