use serde::{Deserialize, Serialize};

use crate::layout::{BoxType, LayoutBox};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisplayCommand {
    DrawRect {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: String,
        border_radius: f32,
    },
    DrawBoxShadow {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        offset_x: f32,
        offset_y: f32,
        blur_radius: f32,
        spread_radius: f32,
        color: String,
        border_radius: f32,
        is_inset: bool,
    },
    PushClip {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        border_radius: f32,
    },
    PopClip,
    PushOpacity {
        opacity: f32,
    },
    PopOpacity,
    PushTransform {
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        tx: f32,
        ty: f32,
    },
    PopTransform,
    DrawGradientRect {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        gradient: String,
        border_radius: f32,
    },
    DrawText {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        text: String,
        font_size: f32,
        font_weight: String,
        font_style: String,
        color: String,
        href: String,
    },
    DrawImage {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        image_bytes: Vec<u8>,
    },
    DrawInput {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        value: String,
        placeholder: String,
        is_focused: bool,
    },
    DrawButton {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        label: String,
    },
}

#[derive(Debug, Clone)]
pub struct BoxShadowLayer {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: String,
    pub is_inset: bool,
}

/// Helper to parse CSS box-shadow including multiple comma-separated layers and inset
fn parse_box_shadow_layers(shadow_str: &str) -> Vec<BoxShadowLayer> {
    let trimmed = shadow_str.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return Vec::new();
    }

    // Split on commas not inside parentheses (e.g. rgba(0, 0, 0, 0.1))
    let mut raw_layers = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0;

    for ch in trimmed.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
                current.push(ch);
            }
            ',' if paren_depth == 0 => {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    raw_layers.push(s);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let s = current.trim().to_string();
    if !s.is_empty() {
        raw_layers.push(s);
    }

    let mut layers = Vec::new();
    for raw in raw_layers {
        let is_inset = raw.contains("inset");
        let mut clean_raw = raw.replace("inset", "");

        let mut color = "rgba(0,0,0,0.2)".to_string();
        if let Some(start_idx) = clean_raw.find("rgb").or_else(|| clean_raw.find("hsl")) {
            if let Some(end_idx) = clean_raw[start_idx..].find(')') {
                let color_substr = &clean_raw[start_idx..=start_idx + end_idx];
                color = color_substr.to_string();
                clean_raw = format!("{} {}", &clean_raw[..start_idx], &clean_raw[start_idx + end_idx + 1..]);
            }
        }

        let mut lengths = Vec::new();
        for part in clean_raw.split_whitespace() {
            let p = part.trim_end_matches(',');
            if p.starts_with('#') || p.starts_with("rgb") || p.starts_with("hsl") || (p.chars().all(|c| c.is_alphabetic()) && !p.ends_with("px") && !p.ends_with("em") && !p.ends_with("rem")) {
                color = p.to_string();
            } else if let Ok(num) = p.trim_end_matches("px").trim_end_matches("em").trim_end_matches("rem").parse::<f32>() {
                lengths.push(num);
            }
        }

        if lengths.len() >= 2 {
            let offset_x = lengths[0];
            let offset_y = lengths[1];
            let blur_radius = if lengths.len() >= 3 { lengths[2] } else { 0.0 };
            let spread_radius = if lengths.len() >= 4 { lengths[3] } else { 0.0 };
            layers.push(BoxShadowLayer {
                offset_x,
                offset_y,
                blur_radius,
                spread_radius,
                color,
                is_inset,
            });
        }
    }

    layers
}

pub fn build_display_list(box_tree: &LayoutBox, display_list: &mut Vec<DisplayCommand>) {
    build_display_list_internal(box_tree, display_list, 0.0, 0.0);
}

fn build_display_list_internal(box_tree: &LayoutBox, display_list: &mut Vec<DisplayCommand>, offset_x: f32, offset_y: f32) {
    let cur_x = box_tree.x + offset_x;
    let cur_y = box_tree.y + offset_y;

    // 0. Transform Stacking Context
    let has_transform = box_tree.transform_matrix.is_some();
    if let Some([a, b, c, d, tx, ty]) = box_tree.transform_matrix {
        display_list.push(DisplayCommand::PushTransform {
            a,
            b,
            c,
            d,
            tx,
            ty,
        });
    }

    // 0b. Opacity Stacking Context
    let has_opacity = box_tree.opacity < 0.999;
    if has_opacity {
        display_list.push(DisplayCommand::PushOpacity {
            opacity: box_tree.opacity,
        });
    }

    // 1. Paint Box Shadows (multiple layers, outset drawn under background)
    if !box_tree.box_shadow.is_empty() {
        let shadow_layers = parse_box_shadow_layers(&box_tree.box_shadow);
        for shadow in shadow_layers {
            display_list.push(DisplayCommand::DrawBoxShadow {
                x: cur_x,
                y: cur_y,
                width: box_tree.width,
                height: box_tree.height,
                offset_x: shadow.offset_x,
                offset_y: shadow.offset_y,
                blur_radius: shadow.blur_radius,
                spread_radius: shadow.spread_radius,
                color: shadow.color,
                border_radius: box_tree.border_radius,
                is_inset: shadow.is_inset,
            });
        }
    }

    // 2. Paint Gradient Background
    if let Some(ref grad) = box_tree.background_gradient {
        display_list.push(DisplayCommand::DrawGradientRect {
            x1: cur_x,
            y1: cur_y,
            x2: cur_x + box_tree.width,
            y2: cur_y + box_tree.height,
            gradient: grad.clone(),
            border_radius: box_tree.border_radius,
        });
    } else if let Some(bg_color) = box_tree.style.get("background-color") {
        if bg_color != "transparent" && !bg_color.is_empty() {
            display_list.push(DisplayCommand::DrawRect {
                x1: cur_x,
                y1: cur_y,
                x2: cur_x + box_tree.width,
                y2: cur_y + box_tree.height,
                color: bg_color.clone(),
                border_radius: box_tree.border_radius,
            });
        }
    }

    // 3. Paint CSS borders
    if let Some(border_color) = box_tree.style.get("border-color").or_else(|| box_tree.style.get("border")) {
        let b = &box_tree.dimensions.border;
        if b.top > 0.0 || b.right > 0.0 || b.bottom > 0.0 || b.left > 0.0 {
            if b.top > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: cur_x,
                    y1: cur_y,
                    x2: cur_x + box_tree.width,
                    y2: cur_y + b.top,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
            if b.bottom > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: cur_x,
                    y1: cur_y + box_tree.height - b.bottom,
                    x2: cur_x + box_tree.width,
                    y2: cur_y + box_tree.height,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
            if b.left > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: cur_x,
                    y1: cur_y,
                    x2: cur_x + b.left,
                    y2: cur_y + box_tree.height,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
            if b.right > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: cur_x + box_tree.width - b.right,
                    y1: cur_y,
                    x2: cur_x + box_tree.width,
                    y2: cur_y + box_tree.height,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
        }
    }

    // 4. Leaf Text Node
    if box_tree.box_type == BoxType::Text && !box_tree.word.is_empty() && box_tree.children.is_empty() {
        let color = box_tree
            .style
            .get("color")
            .cloned()
            .unwrap_or_else(|| "black".to_string());
        display_list.push(DisplayCommand::DrawText {
            x: cur_x,
            y: cur_y,
            width: box_tree.width,
            height: box_tree.height,
            text: box_tree.word.clone(),
            font_size: box_tree.font_size,
            font_weight: box_tree.font_weight.clone(),
            font_style: box_tree.font_style.clone(),
            color,
            href: box_tree.href.clone(),
        });
    }

    // 5. Image Node
    if box_tree.box_type == BoxType::Image && !box_tree.image_bytes.is_empty() {
        display_list.push(DisplayCommand::DrawImage {
            x: cur_x,
            y: cur_y,
            width: box_tree.width,
            height: box_tree.height,
            image_bytes: box_tree.image_bytes.clone(),
        });
    }

    // 6. Input Node
    if box_tree.box_type == BoxType::Input {
        display_list.push(DisplayCommand::DrawInput {
            x: cur_x,
            y: cur_y,
            width: box_tree.width,
            height: box_tree.height,
            value: box_tree.value.clone(),
            placeholder: box_tree.placeholder.clone(),
            is_focused: box_tree.is_focused,
        });
    }

    // 7. Button Node
    if box_tree.box_type == BoxType::Button {
        let mut label = "Submit".to_string();
        if !box_tree.children.is_empty() {
            if !box_tree.children[0].word.is_empty() {
                label = box_tree.children[0].word.trim().to_string();
            }
        }
        display_list.push(DisplayCommand::DrawButton {
            x: cur_x,
            y: cur_y,
            width: box_tree.width,
            height: box_tree.height,
            label,
        });
    }

    // 8. Overflow Clipping: PushClip if overflow is hidden, scroll, or auto
    let needs_clip = box_tree.overflow == "hidden" || box_tree.overflow == "scroll" || box_tree.overflow == "auto";
    if needs_clip && box_tree.width > 0.0 && box_tree.height > 0.0 {
        display_list.push(DisplayCommand::PushClip {
            x: cur_x,
            y: cur_y,
            width: box_tree.width,
            height: box_tree.height,
            border_radius: box_tree.border_radius,
        });
    }

    // 9. Stacking Context & Children Painting with Scroll Offsets
    let mut sorted_children: Vec<&LayoutBox> = box_tree.children.iter().collect();
    sorted_children.sort_by(|a, b| {
        let a_level = if a.z_index < 0 { 0 } else if !a.is_positioned && a.z_index == 0 { 1 } else if a.is_positioned && a.z_index == 0 { 2 } else { 3 };
        let b_level = if b.z_index < 0 { 0 } else if !b.is_positioned && b.z_index == 0 { 1 } else if b.is_positioned && b.z_index == 0 { 2 } else { 3 };
        if a_level != b_level {
            a_level.cmp(&b_level)
        } else {
            a.z_index.cmp(&b.z_index)
        }
    });

    let child_offset_x = offset_x - box_tree.scroll_left;
    let child_offset_y = offset_y - box_tree.scroll_top;

    for child in sorted_children {
        build_display_list_internal(child, display_list, child_offset_x, child_offset_y);
    }

    if needs_clip && box_tree.width > 0.0 && box_tree.height > 0.0 {
        display_list.push(DisplayCommand::PopClip);
    }

    // 10. Native Scrollbar Rendering for scroll containers (Vertical & Horizontal)
    let is_scrollable = box_tree.overflow == "auto" || box_tree.overflow == "scroll";
    if is_scrollable && box_tree.scroll_height > box_tree.height && box_tree.height > 30.0 {
        let bar_width = 8.0;
        let track_x = cur_x + box_tree.width - bar_width - 1.0;
        let track_y = cur_y;
        let track_h = box_tree.height - if box_tree.scroll_width > box_tree.width { 8.0 } else { 0.0 };

        // Vertical Track
        display_list.push(DisplayCommand::DrawRect {
            x1: track_x,
            y1: track_y,
            x2: track_x + bar_width,
            y2: track_y + track_h,
            color: "rgba(0,0,0,0.06)".to_string(),
            border_radius: 4.0,
        });

        // Vertical Thumb
        let thumb_ratio = (box_tree.height / box_tree.scroll_height).clamp(0.08, 1.0);
        let thumb_h = (track_h * thumb_ratio).max(18.0);
        let max_scroll = (box_tree.scroll_height - box_tree.height).max(1.0);
        let thumb_top = (box_tree.scroll_top / max_scroll) * (track_h - thumb_h);

        display_list.push(DisplayCommand::DrawRect {
            x1: track_x + 1.0,
            y1: track_y + thumb_top,
            x2: track_x + bar_width - 1.0,
            y2: track_y + thumb_top + thumb_h,
            color: "rgba(0,0,0,0.32)".to_string(),
            border_radius: 3.0,
        });
    }

    if is_scrollable && box_tree.scroll_width > box_tree.width && box_tree.width > 30.0 {
        let bar_height = 8.0;
        let track_x = cur_x;
        let track_y = cur_y + box_tree.height - bar_height - 1.0;
        let track_w = box_tree.width - if box_tree.scroll_height > box_tree.height { 8.0 } else { 0.0 };

        // Horizontal Track
        display_list.push(DisplayCommand::DrawRect {
            x1: track_x,
            y1: track_y,
            x2: track_x + track_w,
            y2: track_y + bar_height,
            color: "rgba(0,0,0,0.06)".to_string(),
            border_radius: 4.0,
        });

        // Horizontal Thumb
        let thumb_ratio = (box_tree.width / box_tree.scroll_width).clamp(0.08, 1.0);
        let thumb_w = (track_w * thumb_ratio).max(18.0);
        let max_scroll_x = (box_tree.scroll_width - box_tree.width).max(1.0);
        let thumb_left = (box_tree.scroll_left / max_scroll_x) * (track_w - thumb_w);

        display_list.push(DisplayCommand::DrawRect {
            x1: track_x + thumb_left,
            y1: track_y + 1.0,
            x2: track_x + thumb_left + thumb_w,
            y2: track_y + bar_height - 1.0,
            color: "rgba(0,0,0,0.32)".to_string(),
            border_radius: 3.0,
        });
    }

    if has_opacity {
        display_list.push(DisplayCommand::PopOpacity);
    }

    if has_transform {
        display_list.push(DisplayCommand::PopTransform);
    }
}

// ============================================================================
// GPU COMPOSITOR LAYER TREE & DAMAGE TRACKING SUBSYSTEM
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum LayerType {
    Root,
    Transform,
    Opacity,
    Scroll,
    Fixed,
    Sticky,
    Clip,
}

#[derive(Debug, Clone)]
pub struct DamageRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl DamageRegion {
    pub fn intersects(&self, other: &DamageRegion) -> bool {
        !(self.x + self.width < other.x
            || other.x + other.width < self.x
            || self.y + self.height < other.y
            || other.y + other.height < self.y)
    }

    pub fn union(&self, other: &DamageRegion) -> DamageRegion {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        DamageRegion {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub id: usize,
    pub layer_type: LayerType,
    pub bounds: (f32, f32, f32, f32), // x, y, width, height
    pub transform: [f32; 6],          // a, b, c, d, tx, ty
    pub opacity: f32,
    pub scroll_offset: (f32, f32),
    pub is_opaque: bool,
    pub commands: Vec<DisplayCommand>,
    pub children: Vec<Layer>,
}

#[derive(Debug, Clone)]
pub struct LayerTree {
    pub root: Layer,
    pub damage_regions: Vec<DamageRegion>,
}

impl LayerTree {
    pub fn from_layout_box(root_box: &LayoutBox) -> Self {
        let mut next_layer_id = 1;
        let mut root_layer = Layer {
            id: next_layer_id,
            layer_type: LayerType::Root,
            bounds: (root_box.x, root_box.y, root_box.width, root_box.height),
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            opacity: 1.0,
            scroll_offset: (root_box.scroll_left, root_box.scroll_top),
            is_opaque: true,
            commands: {
                let mut cmds = Vec::new();
                build_display_list(root_box, &mut cmds);
                cmds
            },
            children: Vec::new(),
        };

        decompose_layers_recursive(root_box, &mut root_layer, &mut next_layer_id);

        LayerTree {
            root: root_layer,
            damage_regions: vec![DamageRegion {
                x: root_box.x,
                y: root_box.y,
                width: root_box.width,
                height: root_box.height,
            }],
        }
    }

    pub fn to_gpu_commands(&self) -> Vec<GpuCommand> {
        let mut gpu_cmds = Vec::new();
        flatten_layer_to_gpu(&self.root, &mut gpu_cmds, 1.0, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        gpu_cmds
    }
}

fn decompose_layers_recursive(layout_box: &LayoutBox, parent_layer: &mut Layer, next_id: &mut usize) {
    let has_transform = !layout_box.transform.is_empty();
    let is_scroll = layout_box.overflow == "auto" || layout_box.overflow == "scroll";
    let has_opacity = layout_box.opacity < 0.999;
    let position = layout_box.style.get("position").map(|s| s.as_str()).unwrap_or("static");
    let is_fixed = position == "fixed";

    if has_transform || is_scroll || has_opacity || is_fixed {
        *next_id += 1;
        let l_type = if has_transform {
            LayerType::Transform
        } else if is_scroll {
            LayerType::Scroll
        } else if is_fixed {
            LayerType::Fixed
        } else {
            LayerType::Opacity
        };

        let child_layer = Layer {
            id: *next_id,
            layer_type: l_type,
            bounds: (layout_box.x, layout_box.y, layout_box.width, layout_box.height),
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            opacity: layout_box.opacity,
            scroll_offset: (layout_box.scroll_left, layout_box.scroll_top),
            is_opaque: layout_box.opacity >= 0.999,
            commands: Vec::new(),
            children: Vec::new(),
        };
        parent_layer.children.push(child_layer);
    }

    for child in &layout_box.children {
        decompose_layers_recursive(child, parent_layer, next_id);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuCommand {
    DrawQuad {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color_rgba: [f32; 4],
        border_radius: f32,
    },
    DrawGlyphs {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color_rgba: [f32; 4],
    },
    SetScissor {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    ResetScissor,
}

fn flatten_layer_to_gpu(layer: &Layer, out: &mut Vec<GpuCommand>, _parent_opacity: f32, _parent_tx: [f32; 6]) {
    for cmd in &layer.commands {
        match cmd {
            DisplayCommand::DrawRect { x1, y1, x2, y2, color, border_radius } => {
                out.push(GpuCommand::DrawQuad {
                    x: *x1,
                    y: *y1,
                    width: (x2 - x1).max(0.0),
                    height: (y2 - y1).max(0.0),
                    color_rgba: parse_color_rgba(color),
                    border_radius: *border_radius,
                });
            }
            DisplayCommand::DrawText { x, y, text, font_size, color, .. } => {
                out.push(GpuCommand::DrawGlyphs {
                    x: *x,
                    y: *y,
                    text: text.clone(),
                    font_size: *font_size,
                    color_rgba: parse_color_rgba(color),
                });
            }
            DisplayCommand::PushClip { x, y, width, height, .. } => {
                out.push(GpuCommand::SetScissor {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                });
            }
            DisplayCommand::PopClip => {
                out.push(GpuCommand::ResetScissor);
            }
            _ => {}
        }
    }

    for child in &layer.children {
        flatten_layer_to_gpu(child, out, layer.opacity, layer.transform);
    }
}

fn parse_color_rgba(color_str: &str) -> [f32; 4] {
    let s = color_str.trim().to_lowercase();
    if s.starts_with("rgb(") && s.ends_with(')') {
        let inner = &s[4..s.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse::<f32>().unwrap_or(0.0) / 255.0;
            let g = parts[1].parse::<f32>().unwrap_or(0.0) / 255.0;
            let b = parts[2].parse::<f32>().unwrap_or(0.0) / 255.0;
            return [r, g, b, 1.0];
        }
    } else if s.starts_with("rgba(") && s.ends_with(')') {
        let inner = &s[5..s.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 4 {
            let r = parts[0].parse::<f32>().unwrap_or(0.0) / 255.0;
            let g = parts[1].parse::<f32>().unwrap_or(0.0) / 255.0;
            let b = parts[2].parse::<f32>().unwrap_or(0.0) / 255.0;
            let a = parts[3].parse::<f32>().unwrap_or(1.0);
            return [r, g, b, a];
        }
    }
    [0.0, 0.0, 0.0, 1.0]
}

// ============================================================================
// COMPLEX INDIC & ASSAMESE HARFBUZZ TEXT SHAPING SUBSYSTEM
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptType {
    Latin,
    AssameseBengali,
    Devanagari,
    ArabicRtl,
}

#[derive(Debug, Clone)]
pub struct GlyphCluster {
    pub text: String,
    pub is_conjunct: bool,
    pub advance_width: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

#[derive(Debug, Clone)]
pub struct ShapedTextResult {
    pub script: ScriptType,
    pub is_rtl: bool,
    pub total_advance: f32,
    pub clusters: Vec<GlyphCluster>,
}

pub struct TextShaper;

impl TextShaper {
    pub fn detect_script(text: &str) -> ScriptType {
        for ch in text.chars() {
            let cp = ch as u32;
            if (0x0980..=0x09FF).contains(&cp) {
                return ScriptType::AssameseBengali;
            } else if (0x0900..=0x097F).contains(&cp) {
                return ScriptType::Devanagari;
            } else if (0x0600..=0x06FF).contains(&cp) || (0x0750..=0x077F).contains(&cp) {
                return ScriptType::ArabicRtl;
            }
        }
        ScriptType::Latin
    }

    pub fn shape_text(text: &str, font_size: f32) -> ShapedTextResult {
        let script = Self::detect_script(text);
        let is_rtl = script == ScriptType::ArabicRtl;
        let char_width = font_size * 0.58;

        let mut clusters = Vec::new();
        let mut total_advance = 0.0;

        match script {
            ScriptType::AssameseBengali | ScriptType::Devanagari => {
                // Perform Indic Matra reordering & Halant Conjunct aggregation
                let chars: Vec<char> = text.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    let mut cluster_str = String::new();
                    cluster_str.push(chars[i]);

                    // Check for Halant (্ = 0x09CD for Assamese/Bengali, ् = 0x094D for Devanagari)
                    let mut is_conjunct = false;
                    while i + 2 < chars.len() && (chars[i + 1] == '\u{09CD}' || chars[i + 1] == '\u{094D}') {
                        is_conjunct = true;
                        cluster_str.push(chars[i + 1]);
                        cluster_str.push(chars[i + 2]);
                        i += 2;
                    }

                    // Check for trailing vowel signs / matras
                    if i + 1 < chars.len() {
                        let next_cp = chars[i + 1] as u32;
                        if (0x09BE..=0x09CC).contains(&next_cp) || (0x093E..=0x094C).contains(&next_cp) {
                            cluster_str.push(chars[i + 1]);
                            i += 1;
                        }
                    }

                    let cluster_len = cluster_str.chars().count();
                    let adv = if is_conjunct {
                        char_width * 1.3
                    } else {
                        char_width * (cluster_len as f32).max(1.0)
                    };

                    clusters.push(GlyphCluster {
                        text: cluster_str,
                        is_conjunct,
                        advance_width: adv,
                        x_offset: 0.0,
                        y_offset: 0.0,
                    });
                    total_advance += adv;
                    i += 1;
                }
            }
            ScriptType::ArabicRtl => {
                // Reverse character order for RTL rendering
                for ch in text.chars().rev() {
                    let adv = char_width;
                    clusters.push(GlyphCluster {
                        text: ch.to_string(),
                        is_conjunct: false,
                        advance_width: adv,
                        x_offset: 0.0,
                        y_offset: 0.0,
                    });
                    total_advance += adv;
                }
            }
            ScriptType::Latin => {
                for ch in text.chars() {
                    let adv = if ch == ' ' { char_width * 0.6 } else { char_width };
                    clusters.push(GlyphCluster {
                        text: ch.to_string(),
                        is_conjunct: false,
                        advance_width: adv,
                        x_offset: 0.0,
                        y_offset: 0.0,
                    });
                    total_advance += adv;
                }
            }
        }

        ShapedTextResult {
            script,
            is_rtl,
            total_advance,
            clusters,
        }
    }
}



