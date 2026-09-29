use crate::layout::{BoxType, LayoutBox};

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    DrawRect {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: String,
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

pub fn build_display_list(box_tree: &LayoutBox, display_list: &mut Vec<DisplayCommand>) {
    // 1. Paint background
    if let Some(bg_color) = box_tree.style.get("background-color") {
        if bg_color != "transparent" && !bg_color.is_empty() {
            display_list.push(DisplayCommand::DrawRect {
                x1: box_tree.x,
                y1: box_tree.y,
                x2: box_tree.x + box_tree.width,
                y2: box_tree.y + box_tree.height,
                color: bg_color.clone(),
            });
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

    // 6. Recursively paint children
    for child in &box_tree.children {
        build_display_list(child, display_list);
    }
}
