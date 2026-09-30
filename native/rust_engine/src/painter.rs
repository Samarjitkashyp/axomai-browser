use crate::layout::{BoxType, LayoutBox};

#[derive(Debug, Clone)]
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
    },
    PushClip {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        border_radius: f32,
    },
    PopClip,
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

/// Helper to parse CSS box-shadow: "2px 4px 6px 1px rgba(0,0,0,0.2)" or "0 4px 8px #000"
fn parse_box_shadow(shadow_str: &str) -> Option<(f32, f32, f32, f32, String)> {
    let trimmed = shadow_str.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return None;
    }

    let mut color = "rgba(0,0,0,0.2)".to_string();
    let mut lengths = Vec::new();

    // Check for rgba(...) or rgb(...) or hsl(...) color token
    let mut parts_str = trimmed.to_string();
    if let Some(start_idx) = trimmed.find("rgb").or_else(|| trimmed.find("hsl")) {
        if let Some(end_idx) = trimmed[start_idx..].find(')') {
            let color_substr = &trimmed[start_idx..=start_idx + end_idx];
            color = color_substr.to_string();
            parts_str = format!("{} {}", &trimmed[..start_idx], &trimmed[start_idx + end_idx + 1..]);
        }
    }

    for part in parts_str.split_whitespace() {
        let p = part.trim_end_matches(',');
        if p.starts_with('#') || p.starts_with("rgb") || p.starts_with("hsl") || (p.chars().all(|c| c.is_alphabetic()) && !p.ends_with("px") && !p.ends_with("em") && !p.ends_with("rem")) {
            color = p.to_string();
        } else if let Ok(num) = p.trim_end_matches("px").trim_end_matches("em").trim_end_matches("rem").parse::<f32>() {
            lengths.push(num);
        }
    }

    if lengths.len() < 2 {
        return None;
    }

    let offset_x = lengths[0];
    let offset_y = lengths[1];
    let blur = if lengths.len() >= 3 { lengths[2] } else { 0.0 };
    let spread = if lengths.len() >= 4 { lengths[3] } else { 0.0 };

    Some((offset_x, offset_y, blur, spread, color))
}

pub fn build_display_list(box_tree: &LayoutBox, display_list: &mut Vec<DisplayCommand>) {
    // 0. Paint Box Shadow (drawn under background)
    if !box_tree.box_shadow.is_empty() {
        if let Some((ox, oy, blur, spread, color)) = parse_box_shadow(&box_tree.box_shadow) {
            display_list.push(DisplayCommand::DrawBoxShadow {
                x: box_tree.x,
                y: box_tree.y,
                width: box_tree.width,
                height: box_tree.height,
                offset_x: ox,
                offset_y: oy,
                blur_radius: blur,
                spread_radius: spread,
                color,
                border_radius: box_tree.border_radius,
            });
        }
    }

    // 1. Paint background
    if let Some(bg_color) = box_tree.style.get("background-color") {
        if bg_color != "transparent" && !bg_color.is_empty() {
            display_list.push(DisplayCommand::DrawRect {
                x1: box_tree.x,
                y1: box_tree.y,
                x2: box_tree.x + box_tree.width,
                y2: box_tree.y + box_tree.height,
                color: bg_color.clone(),
                border_radius: box_tree.border_radius,
            });
        }
    }

    // 1b. Paint CSS borders
    if let Some(border_color) = box_tree.style.get("border-color").or_else(|| box_tree.style.get("border")) {
        let b = &box_tree.dimensions.border;
        if b.top > 0.0 || b.right > 0.0 || b.bottom > 0.0 || b.left > 0.0 {
            if b.top > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: box_tree.x,
                    y1: box_tree.y,
                    x2: box_tree.x + box_tree.width,
                    y2: box_tree.y + b.top,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
            if b.bottom > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: box_tree.x,
                    y1: box_tree.y + box_tree.height - b.bottom,
                    x2: box_tree.x + box_tree.width,
                    y2: box_tree.y + box_tree.height,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
            if b.left > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: box_tree.x,
                    y1: box_tree.y,
                    x2: box_tree.x + b.left,
                    y2: box_tree.y + box_tree.height,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
            if b.right > 0.0 {
                display_list.push(DisplayCommand::DrawRect {
                    x1: box_tree.x + box_tree.width - b.right,
                    y1: box_tree.y,
                    x2: box_tree.x + box_tree.width,
                    y2: box_tree.y + box_tree.height,
                    color: border_color.clone(),
                    border_radius: box_tree.border_radius,
                });
            }
        }
    }

    // 2. Leaf Text Node
    if box_tree.box_type == BoxType::Text && !box_tree.word.is_empty() && box_tree.children.is_empty() {
        let color = box_tree
            .style
            .get("color")
            .cloned()
            .unwrap_or_else(|| "black".to_string());
        display_list.push(DisplayCommand::DrawText {
            x: box_tree.x,
            y: box_tree.y,
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

    // 3. Image Node
    if box_tree.box_type == BoxType::Image && !box_tree.image_bytes.is_empty() {
        display_list.push(DisplayCommand::DrawImage {
            x: box_tree.x,
            y: box_tree.y,
            width: box_tree.width,
            height: box_tree.height,
            image_bytes: box_tree.image_bytes.clone(),
        });
    }

    // 4. Input Node
    if box_tree.box_type == BoxType::Input {
        display_list.push(DisplayCommand::DrawInput {
            x: box_tree.x,
            y: box_tree.y,
            width: box_tree.width,
            height: box_tree.height,
            value: box_tree.value.clone(),
            placeholder: box_tree.placeholder.clone(),
            is_focused: box_tree.is_focused,
        });
    }

    // 5. Button Node
    if box_tree.box_type == BoxType::Button {
        let mut label = "Submit".to_string();
        if !box_tree.children.is_empty() {
            if !box_tree.children[0].word.is_empty() {
                label = box_tree.children[0].word.trim().to_string();
            }
        }
        display_list.push(DisplayCommand::DrawButton {
            x: box_tree.x,
            y: box_tree.y,
            width: box_tree.width,
            height: box_tree.height,
            label,
        });
    }

    // 6. Overflow Clipping: PushClip if overflow is hidden, scroll, or auto
    let needs_clip = box_tree.overflow == "hidden" || box_tree.overflow == "scroll" || box_tree.overflow == "auto";
    if needs_clip && box_tree.width > 0.0 && box_tree.height > 0.0 {
        display_list.push(DisplayCommand::PushClip {
            x: box_tree.x,
            y: box_tree.y,
            width: box_tree.width,
            height: box_tree.height,
            border_radius: box_tree.border_radius,
        });
    }

    // 7. Stacking Context: Sort children according to CSS 2.1 z-index & positioning rules:
    // (1) Negative z-index children (ascending)
    // (2) In-flow, non-positioned children (DOM order)
    // (3) Positioned children with z-index == 0 / auto
    // (4) Positive z-index children (ascending)
    let mut sorted_children: Vec<&LayoutBox> = box_tree.children.iter().collect();
    sorted_children.sort_by(|a, b| {
        let a_level = if a.z_index < 0 {
            0
        } else if !a.is_positioned && a.z_index == 0 {
            1
        } else if a.is_positioned && a.z_index == 0 {
            2
        } else {
            3
        };

        let b_level = if b.z_index < 0 {
            0
        } else if !b.is_positioned && b.z_index == 0 {
            1
        } else if b.is_positioned && b.z_index == 0 {
            2
        } else {
            3
        };

        if a_level != b_level {
            a_level.cmp(&b_level)
        } else {
            a.z_index.cmp(&b.z_index)
        }
    });

    for child in sorted_children {
        build_display_list(child, display_list);
    }

    if needs_clip && box_tree.width > 0.0 && box_tree.height > 0.0 {
        display_list.push(DisplayCommand::PopClip);
    }
}

