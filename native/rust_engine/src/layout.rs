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

pub fn parse_track_min_width(pattern: &str, available_space: f32) -> f32 {
    let p = pattern.trim();
    if p.starts_with("minmax(") && p.ends_with(')') {
        let inner = &p[7..p.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if !parts.is_empty() {
            return parse_length(parts[0], available_space, 100.0);
        }
    }
    parse_length(p, available_space, 100.0)
}

pub fn parse_grid_line_span(val: &str) -> (Option<usize>, usize) {
    let s = val.trim();
    if s.is_empty() || s == "auto" {
        return (None, 1);
    }
    if s.starts_with("span") {
        let count = s[4..].trim().parse::<usize>().unwrap_or(1).max(1);
        return (None, count);
    }
    if s.contains('/') {
        let parts: Vec<&str> = s.split('/').map(|p| p.trim()).collect();
        let start_opt = parts[0].parse::<usize>().ok();
        let span = if parts.len() > 1 {
            let p2 = parts[1];
            if p2.starts_with("span") {
                p2[4..].trim().parse::<usize>().unwrap_or(1).max(1)
            } else if let Ok(end) = p2.parse::<usize>() {
                if let Some(st) = start_opt {
                    end.saturating_sub(st).max(1)
                } else {
                    1
                }
            } else {
                1
            }
        } else {
            1
        };
        return (start_opt, span);
    }
    if let Ok(st) = s.parse::<usize>() {
        return (Some(st), 1);
    }
    (None, 1)
}

pub fn parse_grid_template_areas(val: &str) -> Vec<Vec<String>> {
    let mut matrix: Vec<Vec<String>> = Vec::new();
    let mut in_quote = false;
    let mut current_row_str = String::new();
    for ch in val.chars() {
        if ch == '"' || ch == '\'' {
            if in_quote {
                let row_tokens: Vec<String> = current_row_str
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                if !row_tokens.is_empty() {
                    matrix.push(row_tokens);
                }
                current_row_str.clear();
                in_quote = false;
            } else {
                in_quote = true;
            }
        } else if in_quote {
            current_row_str.push(ch);
        }
    }
    matrix
}

pub fn find_grid_area_bounds(matrix: &[Vec<String>], area_name: &str) -> Option<(usize, usize, usize, usize)> {
    let mut min_r = usize::MAX;
    let mut max_r = 0;
    let mut min_c = usize::MAX;
    let mut max_c = 0;
    let mut found = false;

    for (r, row) in matrix.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell == area_name {
                min_r = min_r.min(r);
                max_r = max_r.max(r);
                min_c = min_c.min(c);
                max_c = max_c.max(c);
                found = true;
            }
        }
    }

    if found {
        Some((min_c, max_c - min_c + 1, min_r, max_r - min_r + 1))
    } else {
        None
    }
}

pub fn parse_grid_tracks(track_str: &str, available_space: f32, gap: f32) -> Vec<f32> {
    let mut tokens: Vec<String> = Vec::new();
    let trimmed = track_str.trim();
    if trimmed.is_empty() {
        return vec![available_space.max(10.0)];
    }

    let mut s = trimmed;
    while let Some(rep_idx) = s.find("repeat(") {
        let before = &s[..rep_idx];
        for tok in before.split_whitespace() {
            if !tok.is_empty() {
                tokens.push(tok.to_string());
            }
        }
        if let Some(close_idx) = s[rep_idx..].find(')') {
            let inner = &s[rep_idx + 7..rep_idx + close_idx];
            let parts: Vec<&str> = inner.splitn(2, ',').map(|p| p.trim()).collect();
            if parts.len() >= 2 {
                let count_str = parts[0];
                let pattern = parts[1];
                if count_str == "auto-fit" || count_str == "auto-fill" {
                    let min_w = parse_track_min_width(pattern, available_space).max(20.0);
                    let count = ((available_space + gap) / (min_w + gap)).floor().max(1.0) as usize;
                    for _ in 0..count {
                        tokens.push(pattern.to_string());
                    }
                } else if let Ok(count) = count_str.parse::<usize>() {
                    for _ in 0..count {
                        tokens.push(pattern.to_string());
                    }
                } else {
                    tokens.push(pattern.to_string());
                }
            }
            s = &s[rep_idx + close_idx + 1..];
        } else {
            break;
        }
    }
    for tok in s.split_whitespace() {
        if !tok.is_empty() {
            tokens.push(tok.to_string());
        }
    }

    if tokens.is_empty() {
        return vec![available_space.max(10.0)];
    }

    let n = tokens.len();
    let total_gap = (n.saturating_sub(1) as f32) * gap;
    let net_space = (available_space - total_gap).max(0.0);

    let mut fixed_sum: f32 = 0.0;
    let mut total_fr: f32 = 0.0;
    let mut track_kinds: Vec<(Option<f32>, f32)> = Vec::new();

    for tok in &tokens {
        if tok.starts_with("minmax(") && tok.ends_with(')') {
            let inner = &tok[7..tok.len() - 1];
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            let min_val = if !parts.is_empty() { parse_length(parts[0], net_space, 0.0) } else { 0.0 };
            let max_val_str = if parts.len() > 1 { parts[1] } else { "1fr" };
            if max_val_str.ends_with("fr") {
                let fr = max_val_str[..max_val_str.len() - 2].trim().parse::<f32>().unwrap_or(1.0).max(0.1);
                total_fr += fr;
                track_kinds.push((None, fr));
            } else {
                let px = parse_length(max_val_str, net_space, min_val).max(min_val);
                fixed_sum += px;
                track_kinds.push((Some(px), 0.0));
            }
        } else if tok.ends_with("fr") {
            let fr_val = tok[..tok.len() - 2].trim().parse::<f32>().unwrap_or(1.0).max(0.1);
            total_fr += fr_val;
            track_kinds.push((None, fr_val));
        } else if tok.ends_with("px") {
            let px = tok[..tok.len() - 2].trim().parse::<f32>().unwrap_or(0.0);
            fixed_sum += px;
            track_kinds.push((Some(px), 0.0));
        } else if tok.ends_with('%') {
            let pct = tok[..tok.len() - 1].trim().parse::<f32>().unwrap_or(0.0) / 100.0;
            let px = net_space * pct;
            fixed_sum += px;
            track_kinds.push((Some(px), 0.0));
        } else if tok == "auto" {
            total_fr += 1.0;
            track_kinds.push((None, 1.0));
        } else if let Ok(px) = tok.parse::<f32>() {
            fixed_sum += px;
            track_kinds.push((Some(px), 0.0));
        } else {
            total_fr += 1.0;
            track_kinds.push((None, 1.0));
        }
    }

    let remaining_for_fr = (net_space - fixed_sum).max(0.0);
    let fr_unit = if total_fr > 0.0 { remaining_for_fr / total_fr } else { 0.0 };

    track_kinds.into_iter().map(|(fixed, fr)| {
        if let Some(px) = fixed {
            px
        } else {
            (fr * fr_unit).max(10.0)
        }
    }).collect()
}

/// Multiplies two 2D affine transform matrices [a, b, c, d, tx, ty]
pub fn multiply_transforms(m1: [f32; 6], m2: [f32; 6]) -> [f32; 6] {
    let [a1, b1, c1, d1, tx1, ty1] = m1;
    let [a2, b2, c2, d2, tx2, ty2] = m2;

    [
        a1 * a2 + c1 * b2,
        b1 * a2 + d1 * b2,
        a1 * c2 + c1 * d2,
        b1 * c2 + d1 * d2,
        a1 * tx2 + c1 * ty2 + tx1,
        b1 * tx2 + d1 * ty2 + ty1,
    ]
}

/// Computes the inverse of a 2D affine transform matrix [a, b, c, d, tx, ty]
pub fn invert_transform(m: [f32; 6]) -> Option<[f32; 6]> {
    let [a, b, c, d, tx, ty] = m;
    let det = a * d - b * c;
    if det.abs() < 1e-6 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some([
        d * inv_det,
        -b * inv_det,
        -c * inv_det,
        a * inv_det,
        (c * ty - d * tx) * inv_det,
        (b * tx - a * ty) * inv_det,
    ])
}

/// Applies a 2D affine transform matrix to a point (x, y)
pub fn transform_point(m: [f32; 6], x: f32, y: f32) -> (f32, f32) {
    let [a, b, c, d, tx, ty] = m;
    (a * x + c * y + tx, b * x + d * y + ty)
}

pub fn parse_transform_origin(origin_str: Option<&String>, width: f32, height: f32) -> (f32, f32) {
    let s = origin_str.map(|s| s.as_str()).unwrap_or("50% 50%");
    let parts: Vec<&str> = s.split_whitespace().collect();
    let ox = match parts.first().copied().unwrap_or("50%") {
        "left" => 0.0,
        "center" => width * 0.5,
        "right" => width,
        val => parse_length(val, width, width * 0.5),
    };
    let oy = match parts.get(1).copied().unwrap_or("50%") {
        "top" => 0.0,
        "center" => height * 0.5,
        "bottom" => height,
        val => parse_length(val, height, height * 0.5),
    };
    (ox, oy)
}

pub fn apply_transform_origin(matrix: [f32; 6], ox: f32, oy: f32) -> [f32; 6] {
    let t_to_origin = [1.0, 0.0, 0.0, 1.0, -ox, -oy];
    let t_from_origin = [1.0, 0.0, 0.0, 1.0, ox, oy];
    multiply_transforms(t_from_origin, multiply_transforms(matrix, t_to_origin))
}

pub fn resolve_transform_matrix(
    transform_str: &str,
    origin_str: Option<&String>,
    width: f32,
    height: f32,
) -> Option<[f32; 6]> {
    let raw = parse_transform_matrix(transform_str, width, height)?;
    let (ox, oy) = parse_transform_origin(origin_str, width, height);
    Some(apply_transform_origin(raw, ox, oy))
}

pub fn parse_transform_matrix(transform_str: &str, width: f32, height: f32) -> Option<[f32; 6]> {
    let trimmed = transform_str.trim();
    if trimmed.is_empty() || trimmed == "none" {
        return None;
    }

    let mut current_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut has_transform = false;

    let mut remaining = trimmed;
    while let Some(open_paren) = remaining.find('(') {
        let func_name = remaining[..open_paren].trim();
        if let Some(close_paren) = remaining[open_paren..].find(')') {
            let actual_close = open_paren + close_paren;
            let args_str = &remaining[open_paren + 1..actual_close];
            let name = func_name.split_whitespace().last().unwrap_or(func_name);

            let mut m = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            let mut valid_func = true;

            match name {
                "translate" => {
                    let parts: Vec<&str> = args_str.split(',').collect();
                    let tx = if !parts.is_empty() { parse_length(parts[0], width, 0.0) } else { 0.0 };
                    let ty = if parts.len() > 1 { parse_length(parts[1], height, 0.0) } else { 0.0 };
                    m = [1.0, 0.0, 0.0, 1.0, tx, ty];
                }
                "translateX" => {
                    let tx = parse_length(args_str, width, 0.0);
                    m = [1.0, 0.0, 0.0, 1.0, tx, 0.0];
                }
                "translateY" => {
                    let ty = parse_length(args_str, height, 0.0);
                    m = [1.0, 0.0, 0.0, 1.0, 0.0, ty];
                }
                "rotate" => {
                    let arg = args_str.trim();
                    let rad = if arg.ends_with("deg") {
                        arg[..arg.len() - 3].trim().parse::<f32>().unwrap_or(0.0).to_radians()
                    } else if arg.ends_with("rad") {
                        arg[..arg.len() - 3].trim().parse::<f32>().unwrap_or(0.0)
                    } else if arg.ends_with("turn") {
                        arg[..arg.len() - 4].trim().parse::<f32>().unwrap_or(0.0) * std::f32::consts::TAU
                    } else {
                        arg.parse::<f32>().unwrap_or(0.0).to_radians()
                    };
                    m = [rad.cos(), rad.sin(), -rad.sin(), rad.cos(), 0.0, 0.0];
                }
                "scale" => {
                    let parts: Vec<&str> = args_str.split(',').collect();
                    let sx = if !parts.is_empty() { parts[0].trim().parse::<f32>().unwrap_or(1.0) } else { 1.0 };
                    let sy = if parts.len() > 1 { parts[1].trim().parse::<f32>().unwrap_or(sx) } else { sx };
                    m = [sx, 0.0, 0.0, sy, 0.0, 0.0];
                }
                "scaleX" => {
                    let sx = args_str.trim().parse::<f32>().unwrap_or(1.0);
                    m = [sx, 0.0, 0.0, 1.0, 0.0, 0.0];
                }
                "scaleY" => {
                    let sy = args_str.trim().parse::<f32>().unwrap_or(1.0);
                    m = [1.0, 0.0, 0.0, sy, 0.0, 0.0];
                }
                "skew" => {
                    let parts: Vec<&str> = args_str.split(',').collect();
                    let ax = if !parts.is_empty() {
                        let a = parts[0].trim();
                        if a.ends_with("deg") { a[..a.len() - 3].trim().parse::<f32>().unwrap_or(0.0).to_radians() } else { a.parse::<f32>().unwrap_or(0.0).to_radians() }
                    } else { 0.0 };
                    let ay = if parts.len() > 1 {
                        let a = parts[1].trim();
                        if a.ends_with("deg") { a[..a.len() - 3].trim().parse::<f32>().unwrap_or(0.0).to_radians() } else { a.parse::<f32>().unwrap_or(0.0).to_radians() }
                    } else { 0.0 };
                    m = [1.0, ay.tan(), ax.tan(), 1.0, 0.0, 0.0];
                }
                "skewX" => {
                    let a = args_str.trim();
                    let rad = if a.ends_with("deg") { a[..a.len() - 3].trim().parse::<f32>().unwrap_or(0.0).to_radians() } else { a.parse::<f32>().unwrap_or(0.0).to_radians() };
                    m = [1.0, 0.0, rad.tan(), 1.0, 0.0, 0.0];
                }
                "skewY" => {
                    let a = args_str.trim();
                    let rad = if a.ends_with("deg") { a[..a.len() - 3].trim().parse::<f32>().unwrap_or(0.0).to_radians() } else { a.parse::<f32>().unwrap_or(0.0).to_radians() };
                    m = [1.0, rad.tan(), 0.0, 1.0, 0.0, 0.0];
                }
                "matrix" => {
                    let parts: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 6 {
                        if let (Ok(m_a), Ok(m_b), Ok(m_c), Ok(m_d), Ok(m_tx), Ok(m_ty)) = (
                            parts[0].parse::<f32>(),
                            parts[1].parse::<f32>(),
                            parts[2].parse::<f32>(),
                            parts[3].parse::<f32>(),
                            parts[4].parse::<f32>(),
                            parts[5].parse::<f32>(),
                        ) {
                            m = [m_a, m_b, m_c, m_d, m_tx, m_ty];
                        }
                    }
                }
                _ => {
                    valid_func = false;
                }
            }

            if valid_func {
                current_matrix = multiply_transforms(current_matrix, m);
                has_transform = true;
            }

            remaining = &remaining[actual_close + 1..];
        } else {
            break;
        }
    }

    if has_transform {
        Some(current_matrix)
    } else {
        None
    }
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
    Grid,
    Table,
    TableRow,
    TableCell,
    TableSection,
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
    pub z_index: i32,
    pub is_positioned: bool,
    pub overflow: String,
    pub border_radius: f32,
    pub box_shadow: String,
    pub opacity: f32,
    pub scroll_top: f32,
    pub scroll_left: f32,
    pub scroll_height: f32,
    pub scroll_width: f32,
    pub is_scroll_container: bool,
    pub transform: String,
    pub transform_matrix: Option<[f32; 6]>,
    pub background_gradient: Option<String>,
    pub id: String,
    pub class_name: String,
    pub tag_name: String,
    pub node_id: usize,
}

impl LayoutBox {
    pub fn new(box_type: BoxType, style: HashMap<String, String>, href: String) -> Self {
        let font_size = parse_px(style.get("font-size").map(|s| s.as_str()).unwrap_or("16px"), 16.0);
        let font_weight = style.get("font-weight").cloned().unwrap_or_else(|| "normal".to_string());
        let font_style = style.get("font-style").cloned().unwrap_or_else(|| "normal".to_string());
        let font_family = style.get("font-family").cloned().unwrap_or_else(|| "sans-serif".to_string());

        let z_index = style.get("z-index").and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(0);
        let pos = style.get("position").map(|s| s.as_str()).unwrap_or("static");
        let is_positioned = pos == "relative" || pos == "absolute" || pos == "fixed" || pos == "sticky";
        let overflow = style.get("overflow").cloned().unwrap_or_else(|| "visible".to_string());
        let border_radius = parse_px(style.get("border-radius").map(|s| s.as_str()).unwrap_or("0px"), 0.0);
        let box_shadow = style.get("box-shadow").cloned().unwrap_or_default();
        let opacity = style.get("opacity").and_then(|s| s.trim().parse::<f32>().ok()).unwrap_or(1.0).clamp(0.0, 1.0);
        let transform = style.get("transform").cloned().unwrap_or_default();

        let background_gradient = style.get("background").or_else(|| style.get("background-image")).and_then(|bg| {
            if bg.contains("gradient") {
                Some(bg.clone())
            } else {
                None
            }
        });

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
            z_index,
            is_positioned,
            overflow,
            border_radius,
            box_shadow,
            opacity,
            scroll_top: 0.0,
            scroll_left: 0.0,
            scroll_height: 0.0,
            scroll_width: 0.0,
            is_scroll_container: false,
            transform,
            transform_matrix: None,
            background_gradient,
            id: String::new(),
            class_name: String::new(),
            tag_name: String::new(),
            node_id: 0,
        }
    }

    pub fn layout(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        self.dimensions.padding = parse_edges(&self.style, "padding", max_width);
        self.dimensions.margin = parse_edges(&self.style, "margin", max_width);
        self.dimensions.border = parse_border_properties(&mut self.style, max_width);

        match self.box_type {
            BoxType::Table => self.layout_table(x, y, max_width),
            BoxType::TableRow => self.layout_block(x, y, max_width),
            BoxType::TableCell => self.layout_block(x, y, max_width),
            BoxType::TableSection => self.layout_block(x, y, max_width),
            BoxType::Grid => self.layout_grid(x, y, max_width),
            BoxType::Flex => self.layout_flex(x, y, max_width),
            BoxType::Block | BoxType::AnonymousBlock => self.layout_block(x, y, max_width),
            BoxType::Image | BoxType::Input | BoxType::Button => self.layout_leaf(x, y),
            _ => self.layout_block(x, y, max_width),
        }
    }

    fn layout_table(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        let content_width = (max_width - self.dimensions.margin.left - self.dimensions.margin.right
            - self.dimensions.border.left - self.dimensions.border.right
            - self.dimensions.padding.left - self.dimensions.padding.right).max(0.0);

        self.x = x + self.dimensions.margin.left;
        self.y = y + self.dimensions.margin.top;
        self.width = max_width;

        let start_x = self.x + self.dimensions.border.left + self.dimensions.padding.left;
        let mut cursor_y = self.y + self.dimensions.border.top + self.dimensions.padding.top;

        // 1. Gather all row boxes (either direct children or inside thead/tbody/tfoot)
        let mut row_indices = Vec::new();
        for (i, child) in self.children.iter().enumerate() {
            if child.box_type == BoxType::TableRow || child.tag_name == "tr" {
                row_indices.push((None, i));
            } else if child.box_type == BoxType::TableSection || child.tag_name == "tbody" || child.tag_name == "thead" || child.tag_name == "tfoot" {
                for (sub_i, sub_child) in child.children.iter().enumerate() {
                    if sub_child.box_type == BoxType::TableRow || sub_child.tag_name == "tr" {
                        row_indices.push((Some(i), sub_i));
                    }
                }
            }
        }

        if row_indices.is_empty() {
            return self.layout_block(x, y, max_width);
        }

        // 2. Count maximum columns across all rows
        let mut max_cols = 1;
        for &(sec_idx, row_idx) in &row_indices {
            let row = match sec_idx {
                Some(s) => &self.children[s].children[row_idx],
                None => &self.children[row_idx],
            };
            let cell_count = row.children.iter().filter(|c| {
                c.box_type == BoxType::TableCell || c.tag_name == "td" || c.tag_name == "th" || c.box_type == BoxType::Block || c.box_type == BoxType::Inline
            }).count();
            max_cols = max_cols.max(cell_count);
        }

        let spacing = parse_px(self.style.get("border-spacing").map(|s| s.as_str()).unwrap_or("2px"), 2.0);
        let col_width = ((content_width - (max_cols.saturating_sub(1) as f32) * spacing) / (max_cols as f32)).max(20.0);

        // 3. Layout each row and its cells
        for (sec_idx, row_idx) in row_indices {
            let row = match sec_idx {
                Some(s) => &mut self.children[s].children[row_idx],
                None => &mut self.children[row_idx],
            };

            row.x = start_x;
            row.y = cursor_y;
            row.width = content_width;

            let mut row_cursor_x = start_x;
            let mut row_max_h: f32 = 24.0;

            for cell in &mut row.children {
                cell.dimensions.padding = parse_edges(&cell.style, "padding", col_width);
                cell.dimensions.border = parse_border_properties(&mut cell.style, col_width);
                cell.x = row_cursor_x;
                cell.y = cursor_y;
                cell.width = col_width;

                let cell_h = cell.layout(row_cursor_x, cursor_y, col_width);
                row_max_h = row_max_h.max(cell_h);
                row_cursor_x += col_width + spacing;
            }

            row.height = row_max_h;
            cursor_y += row_max_h + spacing;
        }

        let total_h = cursor_y - y + self.dimensions.border.bottom + self.dimensions.padding.bottom + self.dimensions.margin.bottom;
        self.height = total_h;
        self.dimensions.content.height = (cursor_y - self.y).max(0.0);
        total_h
    }

    pub fn collect_layout_boxes_geometry(&self, map: &mut HashMap<String, (f32, f32, f32, f32, f32, f32)>) {
        if !self.id.is_empty() {
            map.insert(
                format!("#{}", self.id),
                (self.x, self.y, self.width, self.height, self.scroll_width, self.scroll_height),
            );
            map.insert(
                self.id.clone(),
                (self.x, self.y, self.width, self.height, self.scroll_width, self.scroll_height),
            );
        }
        if !self.class_name.is_empty() {
            for cls in self.class_name.split_whitespace() {
                map.entry(format!(".{}", cls)).or_insert((
                    self.x, self.y, self.width, self.height, self.scroll_width, self.scroll_height,
                ));
            }
        }
        if !self.tag_name.is_empty() {
            map.entry(self.tag_name.clone()).or_insert((
                self.x, self.y, self.width, self.height, self.scroll_width, self.scroll_height,
            ));
        }
        if self.node_id > 0 {
            map.insert(
                format!("__node_{}", self.node_id),
                (self.x, self.y, self.width, self.height, self.scroll_width, self.scroll_height),
            );
        }

        for child in &self.children {
            child.collect_layout_boxes_geometry(map);
        }
    }

    pub fn offset_box_and_descendants(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
        self.dimensions.content.x += dx;
        self.dimensions.content.y += dy;
        for child in &mut self.children {
            child.offset_box_and_descendants(dx, dy);
        }
    }

    fn layout_grid(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        let is_border_box = self.style.get("box-sizing").map(|s| s.as_str() == "border-box").unwrap_or(false);
        let explicit_w = self.style.get("width").map(|w| parse_length(w, max_width, -1.0)).unwrap_or(-1.0);

        let content_width = if explicit_w >= 0.0 {
            if is_border_box {
                (explicit_w
                    - self.dimensions.padding.left
                    - self.dimensions.padding.right
                    - self.dimensions.border.left
                    - self.dimensions.border.right)
                    .max(0.0)
            } else {
                explicit_w
            }
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

        self.dimensions.content.x = x + self.dimensions.margin.left + self.dimensions.border.left + self.dimensions.padding.left;
        self.dimensions.content.y = y + self.dimensions.margin.top + self.dimensions.border.top + self.dimensions.padding.top;
        self.dimensions.content.width = content_width;

        let col_gap_str = self.style.get("column-gap").or_else(|| self.style.get("grid-column-gap")).or_else(|| self.style.get("gap")).map(|s| s.as_str()).unwrap_or("0px");
        let row_gap_str = self.style.get("row-gap").or_else(|| self.style.get("grid-row-gap")).or_else(|| self.style.get("gap")).map(|s| s.as_str()).unwrap_or("0px");
        let col_gap = parse_px(col_gap_str, 0.0);
        let row_gap = parse_px(row_gap_str, 0.0);

        let col_template = self.style.get("grid-template-columns").map(|s| s.as_str()).unwrap_or("1fr");
        let col_widths = parse_grid_tracks(col_template, content_width, col_gap);
        let num_cols = col_widths.len().max(1);

        let areas_str_opt = self.style.get("grid-template-areas").map(|s| s.as_str());
        let template_areas = areas_str_opt.map(parse_grid_template_areas).unwrap_or_default();

        let auto_flow = self.style.get("grid-auto-flow").map(|s| s.as_str()).unwrap_or("row");
        let is_dense = auto_flow.contains("dense");

        let mut col_offsets: Vec<f32> = Vec::with_capacity(num_cols);
        let mut acc_col_x = 0.0;
        for (c, &cw) in col_widths.iter().enumerate() {
            col_offsets.push(acc_col_x);
            acc_col_x += cw + if c + 1 < num_cols { col_gap } else { 0.0 };
        }

        let start_x = self.dimensions.content.x;
        let start_y = self.dimensions.content.y;

        let mut occupied: Vec<Vec<bool>> = Vec::new();
        let mut placements: Vec<(usize, usize, usize, usize, usize)> = Vec::with_capacity(self.children.len());

        let mut auto_cursor_row = 0;
        let mut auto_cursor_col = 0;

        for (idx, child) in self.children.iter().enumerate() {
            let mut area_col_opt = None;
            let mut area_row_opt = None;
            if let Some(ga) = child.style.get("grid-area") {
                let ga_clean = ga.trim();
                if !template_areas.is_empty() {
                    if let Some((ac_start, ac_span, ar_start, ar_span)) = find_grid_area_bounds(&template_areas, ga_clean) {
                        area_col_opt = Some((Some(ac_start + 1), ac_span));
                        area_row_opt = Some((Some(ar_start + 1), ar_span));
                    }
                }
            }

            let (explicit_col_start_1, col_span_raw) = area_col_opt.unwrap_or_else(|| {
                let col_prop = child.style.get("grid-column").or_else(|| child.style.get("grid-column-start")).map(|s| s.as_str()).unwrap_or("auto");
                parse_grid_line_span(col_prop)
            });
            let col_span = col_span_raw.max(1).min(num_cols);

            let (explicit_row_start_1, row_span_raw) = area_row_opt.unwrap_or_else(|| {
                let row_prop = child.style.get("grid-row").or_else(|| child.style.get("grid-row-start")).map(|s| s.as_str()).unwrap_or("auto");
                parse_grid_line_span(row_prop)
            });
            let row_span = row_span_raw.max(1);

            let (col_start, row_start) = match (explicit_col_start_1, explicit_row_start_1) {
                (Some(c1), Some(r1)) => {
                    (c1.saturating_sub(1).min(num_cols.saturating_sub(1)), r1.saturating_sub(1))
                }
                (Some(c1), None) => {
                    let c0 = c1.saturating_sub(1).min(num_cols.saturating_sub(1));
                    let mut r0 = if is_dense { 0 } else { auto_cursor_row };
                    loop {
                        while occupied.len() <= r0 + row_span {
                            occupied.push(vec![false; num_cols]);
                        }
                        let mut fits = true;
                        if c0 + col_span > num_cols {
                            fits = false;
                        } else {
                            for dr in 0..row_span {
                                for dc in 0..col_span {
                                    if occupied[r0 + dr][c0 + dc] {
                                        fits = false;
                                        break;
                                    }
                                }
                                if !fits { break; }
                            }
                        }
                        if fits { break; }
                        r0 += 1;
                    }
                    (c0, r0)
                }
                (None, Some(r1)) => {
                    let r0 = r1.saturating_sub(1);
                    while occupied.len() <= r0 + row_span {
                        occupied.push(vec![false; num_cols]);
                    }
                    let mut c0 = 0;
                    loop {
                        if c0 + col_span > num_cols {
                            break;
                        }
                        let mut fits = true;
                        for dr in 0..row_span {
                            for dc in 0..col_span {
                                if occupied[r0 + dr][c0 + dc] {
                                    fits = false;
                                    break;
                                }
                            }
                            if !fits { break; }
                        }
                        if fits { break; }
                        c0 += 1;
                    }
                    (c0.min(num_cols.saturating_sub(col_span)), r0)
                }
                (None, None) => {
                    let mut r0 = if is_dense { 0 } else { auto_cursor_row };
                    let mut c0 = if is_dense { 0 } else { auto_cursor_col };
                    loop {
                        if c0 + col_span > num_cols {
                            r0 += 1;
                            c0 = 0;
                        }
                        while occupied.len() <= r0 + row_span {
                            occupied.push(vec![false; num_cols]);
                        }
                        let mut fits = true;
                        for dr in 0..row_span {
                            for dc in 0..col_span {
                                if occupied[r0 + dr][c0 + dc] {
                                    fits = false;
                                    break;
                                }
                            }
                            if !fits { break; }
                        }
                        if fits {
                            if !is_dense {
                                auto_cursor_row = r0;
                                auto_cursor_col = c0 + col_span;
                            }
                            break;
                        }
                        c0 += 1;
                    }
                    (c0, r0)
                }
            };

            while occupied.len() <= row_start + row_span {
                occupied.push(vec![false; num_cols]);
            }
            for dr in 0..row_span {
                for dc in 0..col_span {
                    if col_start + dc < num_cols {
                        occupied[row_start + dr][col_start + dc] = true;
                    }
                }
            }

            placements.push((idx, col_start, col_span, row_start, row_span));
        }

        let num_rows = occupied.len().max(1);

        let row_template_opt = self.style.get("grid-template-rows").map(|s| s.as_str());
        let explicit_row_heights = row_template_opt.map(|rt| parse_grid_tracks(rt, 0.0, row_gap));

        let mut row_heights: Vec<f32> = vec![0.0; num_rows];
        if let Some(ref erh) = explicit_row_heights {
            for (r, &h) in erh.iter().enumerate().take(num_rows) {
                row_heights[r] = h;
            }
        }

        for &(idx, col_start, col_span, row_start, row_span) in &placements {
            let cell_x = start_x + col_offsets.get(col_start).cloned().unwrap_or(0.0);
            let cell_w = (0..col_span).fold(0.0, |acc, dc| {
                acc + col_widths.get(col_start + dc).cloned().unwrap_or(0.0)
            }) + (col_span.saturating_sub(1) as f32) * col_gap;

            let child = &mut self.children[idx];
            child.width = cell_w;
            let child_h = child.layout(cell_x, start_y, cell_w);

            if explicit_row_heights.is_none() || row_heights[row_start] < child_h {
                if row_span == 1 {
                    row_heights[row_start] = row_heights[row_start].max(child_h);
                } else {
                    let per_row = child_h / (row_span as f32);
                    for dr in 0..row_span {
                        if row_start + dr < num_rows {
                            row_heights[row_start + dr] = row_heights[row_start + dr].max(per_row);
                        }
                    }
                }
            }
        }

        let mut row_offsets: Vec<f32> = Vec::with_capacity(num_rows);
        let mut acc_row_y = 0.0;
        for (r, &rh) in row_heights.iter().enumerate() {
            row_offsets.push(acc_row_y);
            acc_row_y += rh + if r + 1 < num_rows { row_gap } else { 0.0 };
        }

        // Alignment resolution: justify-items / align-items / place-items
        let default_justify_items = self.style.get("justify-items").map(|s| s.as_str()).unwrap_or("stretch");
        let default_align_items = self.style.get("align-items").map(|s| s.as_str()).unwrap_or("stretch");

        let (resolved_align_items, resolved_justify_items) = if let Some(pi) = self.style.get("place-items") {
            let parts: Vec<&str> = pi.split_whitespace().collect();
            let ai = parts.first().copied().unwrap_or("stretch");
            let ji = parts.get(1).copied().unwrap_or(ai);
            (ai, ji)
        } else {
            (default_align_items, default_justify_items)
        };

        let justify_content = self.style.get("justify-content").map(|s| s.as_str()).unwrap_or("start");
        let align_content = self.style.get("align-content").map(|s| s.as_str()).unwrap_or("start");

        let total_cols_w = acc_col_x;
        let free_x = (content_width - total_cols_w).max(0.0);
        let col_shift_x = match justify_content {
            "center" => free_x / 2.0,
            "end" | "flex-end" | "right" => free_x,
            _ => 0.0,
        };

        let explicit_h = self.style.get("height").map(|h| parse_length(h, 0.0, -1.0)).unwrap_or(-1.0);
        let free_y = if explicit_h > acc_row_y { explicit_h - acc_row_y } else { 0.0 };
        let row_shift_y = match align_content {
            "center" => free_y / 2.0,
            "end" | "flex-end" | "bottom" => free_y,
            _ => 0.0,
        };

        for &(idx, col_start, col_span, row_start, row_span) in &placements {
            let child = &mut self.children[idx];
            let cell_x = start_x + col_shift_x + col_offsets.get(col_start).cloned().unwrap_or(0.0);
            let cell_w = (0..col_span).fold(0.0, |acc, dc| {
                acc + col_widths.get(col_start + dc).cloned().unwrap_or(0.0)
            }) + (col_span.saturating_sub(1) as f32) * col_gap;

            let cell_y = start_y + row_shift_y + row_offsets.get(row_start).cloned().unwrap_or(0.0);
            let cell_h = (0..row_span).fold(0.0, |acc, dr| {
                acc + row_heights.get(row_start + dr).cloned().unwrap_or(0.0)
            }) + (row_span.saturating_sub(1) as f32) * row_gap;

            let j_self = child.style.get("justify-self").map(|s| s.as_str()).unwrap_or(resolved_justify_items);
            let a_self = child.style.get("align-self").map(|s| s.as_str()).unwrap_or(resolved_align_items);

            let item_target_x = match j_self {
                "center" => cell_x + (cell_w - child.width).max(0.0) / 2.0,
                "end" | "flex-end" | "right" => cell_x + (cell_w - child.width).max(0.0),
                _ => cell_x,
            };

            let item_target_y = match a_self {
                "center" => cell_y + (cell_h - child.height).max(0.0) / 2.0,
                "end" | "flex-end" | "bottom" => cell_y + (cell_h - child.height).max(0.0),
                _ => cell_y,
            };

            let dx = item_target_x - child.x;
            let dy = item_target_y - child.y;
            if dx != 0.0 || dy != 0.0 {
                child.offset_box_and_descendants(dx, dy);
            }
        }

        let total_grid_height = acc_row_y;
        let final_content_height = if explicit_h >= 0.0 { explicit_h } else { total_grid_height };
        self.dimensions.content.height = final_content_height;

        let border_box = self.dimensions.border_box();
        self.x = border_box.x;
        self.y = border_box.y;
        self.width = border_box.width;
        self.height = border_box.height;

        self.scroll_height = total_grid_height + self.dimensions.padding.top + self.dimensions.padding.bottom + self.dimensions.border.top + self.dimensions.border.bottom;
        self.scroll_width = content_width + self.dimensions.padding.left + self.dimensions.padding.right + self.dimensions.border.left + self.dimensions.border.right;
        let is_scroll_overflow = self.overflow == "auto" || self.overflow == "scroll" || self.overflow == "hidden";
        self.is_scroll_container = is_scroll_overflow && (self.scroll_height > self.height || self.scroll_width > self.width);

        if !self.transform.is_empty() {
            self.transform_matrix = resolve_transform_matrix(&self.transform, self.style.get("transform-origin"), self.width, self.height);
        }

        self.apply_position_offsets(x, y, max_width, self.dimensions.content.height);

        self.dimensions.margin_box().height
    }

    fn layout_block(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        let is_border_box = self.style.get("box-sizing").map(|s| s.as_str() == "border-box").unwrap_or(false);
        let explicit_w = self.style.get("width").map(|w| parse_length(w, max_width, -1.0)).unwrap_or(-1.0);

        let mut content_width = if explicit_w >= 0.0 {
            if is_border_box {
                (explicit_w
                    - self.dimensions.padding.left
                    - self.dimensions.padding.right
                    - self.dimensions.border.left
                    - self.dimensions.border.right)
                    .max(0.0)
            } else {
                explicit_w
            }
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

        // Apply min-width and max-width constraints
        if let Some(min_w_str) = self.style.get("min-width") {
            let min_w = parse_length(min_w_str, max_width, 0.0);
            content_width = content_width.max(min_w);
        }
        if let Some(max_w_str) = self.style.get("max-width") {
            let max_w = parse_length(max_w_str, max_width, f32::MAX);
            content_width = content_width.min(max_w);
        }

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
        let mut final_content_height = if explicit_h >= 0.0 {
            if is_border_box {
                (explicit_h
                    - self.dimensions.padding.top
                    - self.dimensions.padding.bottom
                    - self.dimensions.border.top
                    - self.dimensions.border.bottom)
                    .max(0.0)
            } else {
                explicit_h
            }
        } else {
            computed_content_height
        };

        if let Some(min_h_str) = self.style.get("min-height") {
            let min_h = parse_length(min_h_str, 0.0, 0.0);
            final_content_height = final_content_height.max(min_h);
        }
        if let Some(max_h_str) = self.style.get("max-height") {
            let max_h = parse_length(max_h_str, 0.0, f32::MAX);
            final_content_height = final_content_height.min(max_h);
        }
        self.dimensions.content.height = final_content_height;

        let border_box = self.dimensions.border_box();
        self.x = border_box.x;
        self.y = border_box.y;
        self.width = border_box.width;
        self.height = border_box.height;

        self.scroll_height = computed_content_height + self.dimensions.padding.top + self.dimensions.padding.bottom + self.dimensions.border.top + self.dimensions.border.bottom;
        self.scroll_width = content_width + self.dimensions.padding.left + self.dimensions.padding.right + self.dimensions.border.left + self.dimensions.border.right;
        let is_scroll_overflow = self.overflow == "auto" || self.overflow == "scroll" || self.overflow == "hidden";
        self.is_scroll_container = is_scroll_overflow && (self.scroll_height > self.height || self.scroll_width > self.width);

        if !self.transform.is_empty() {
            self.transform_matrix = resolve_transform_matrix(&self.transform, self.style.get("transform-origin"), self.width, self.height);
        }

        // Apply positioning offsets
        self.apply_position_offsets(x, y, max_width, self.dimensions.content.height);

        self.dimensions.margin_box().height
    }

    fn layout_flex(&mut self, x: f32, y: f32, max_width: f32) -> f32 {
        let is_border_box = self.style.get("box-sizing").map(|s| s.as_str() == "border-box").unwrap_or(false);
        let explicit_w = self.style.get("width").map(|w| parse_length(w, max_width, -1.0)).unwrap_or(-1.0);
        let mut content_width = if explicit_w >= 0.0 {
            if is_border_box {
                (explicit_w
                    - self.dimensions.padding.left
                    - self.dimensions.padding.right
                    - self.dimensions.border.left
                    - self.dimensions.border.right)
                    .max(0.0)
            } else {
                explicit_w
            }
        } else {
            (max_width - self.dimensions.margin.left - self.dimensions.margin.right - self.dimensions.padding.left - self.dimensions.padding.right - self.dimensions.border.left - self.dimensions.border.right).max(0.0)
        };

        if let Some(min_w_str) = self.style.get("min-width") {
            let min_w = parse_length(min_w_str, max_width, 0.0);
            content_width = content_width.max(min_w);
        }
        if let Some(max_w_str) = self.style.get("max-width") {
            let max_w = parse_length(max_w_str, max_width, f32::MAX);
            content_width = content_width.min(max_w);
        }

        self.dimensions.content.x = x + self.dimensions.margin.left + self.dimensions.border.left + self.dimensions.padding.left;
        self.dimensions.content.y = y + self.dimensions.margin.top + self.dimensions.border.top + self.dimensions.padding.top;
        self.dimensions.content.width = content_width;

        let flex_dir = self.style.get("flex-direction").map(|s| s.as_str()).unwrap_or("row");
        let justify = self.style.get("justify-content").map(|s| s.as_str()).unwrap_or("flex-start");
        let align_items = self.style.get("align-items").map(|s| s.as_str()).unwrap_or("stretch");
        let align_content = self.style.get("align-content").map(|s| s.as_str()).unwrap_or("stretch");
        let flex_wrap = self.style.get("flex-wrap").map(|s| s.as_str()).unwrap_or("nowrap");
        let gap = parse_px(self.style.get("gap").map(|s| s.as_str()).unwrap_or("0px"), 0.0);

        let is_row = flex_dir == "row" || flex_dir == "row-reverse";
        let is_reverse = flex_dir == "row-reverse" || flex_dir == "column-reverse";

        let num_children = self.children.len();
        if num_children == 0 {
            return self.dimensions.margin_box().height;
        }

        // Sort children indices by CSS `order` property
        let mut ordered_indices: Vec<(i32, usize)> = (0..num_children)
            .map(|i| {
                let order = self.children[i].style.get("order").and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(0);
                (order, i)
            })
            .collect();
        ordered_indices.sort_by_key(|&(o, idx)| (o, idx));

        let mut child_order: Vec<usize> = ordered_indices.into_iter().map(|(_, idx)| idx).collect();
        if is_reverse {
            child_order.reverse();
        }

        let explicit_h = self.style.get("height").map(|h| parse_length(h, 0.0, -1.0)).unwrap_or(-1.0);

        // Pass 1: Measure hypothetical sizes and factors
        struct FlexItemInfo {
            child_idx: usize,
            base_main_size: f32,
            measured_cross_size: f32,
            flex_grow: f32,
            flex_shrink: f32,
            final_main_size: f32,
        }

        let mut item_infos: Vec<FlexItemInfo> = Vec::new();

        for &idx in &child_order {
            let child = &mut self.children[idx];
            let grow = child.style.get("flex-grow").and_then(|s| s.trim().parse::<f32>().ok()).unwrap_or(0.0);
            let shrink = child.style.get("flex-shrink").and_then(|s| s.trim().parse::<f32>().ok()).unwrap_or(1.0);

            let flex_basis_opt = child.style.get("flex-basis").map(|s| parse_length(s, content_width, -1.0));
            let explicit_w_opt = child.style.get("width").map(|w| parse_length(w, content_width, -1.0));
            let explicit_h_opt = child.style.get("height").map(|h| parse_length(h, 0.0, -1.0));

            let base_main = if is_row {
                if let Some(fb) = flex_basis_opt {
                    if fb >= 0.0 { fb } else { explicit_w_opt.unwrap_or(100.0) }
                } else if let Some(w) = explicit_w_opt {
                    if w >= 0.0 { w } else { 100.0 }
                } else {
                    100.0
                }
            } else {
                explicit_h_opt.unwrap_or(30.0)
            };

            let ch_h = child.layout(self.dimensions.content.x, self.dimensions.content.y, base_main);
            let ch_w = if child.width > 0.0 { child.width } else { base_main };

            let (main_sz, cross_sz) = if is_row { (ch_w, ch_h) } else { (ch_h, ch_w) };

            item_infos.push(FlexItemInfo {
                child_idx: idx,
                base_main_size: main_sz,
                measured_cross_size: cross_sz,
                flex_grow: grow,
                flex_shrink: shrink,
                final_main_size: main_sz,
            });
        }

        // Group into flex lines (supports `flex-wrap: wrap`)
        let mut flex_lines: Vec<Vec<usize>> = Vec::new();
        let mut current_line: Vec<usize> = Vec::new();
        let mut current_line_main = 0.0;

        let container_main_limit = if is_row { content_width } else { if explicit_h >= 0.0 { explicit_h } else { f32::MAX } };

        for (info_idx, info) in item_infos.iter().enumerate() {
            let item_space = if current_line.is_empty() { info.base_main_size } else { info.base_main_size + gap };
            if flex_wrap == "wrap" && current_line_main + item_space > container_main_limit && !current_line.is_empty() {
                flex_lines.push(current_line);
                current_line = Vec::new();
                current_line_main = 0.0;
            }
            current_line.push(info_idx);
            current_line_main += if current_line.len() == 1 { info.base_main_size } else { info.base_main_size + gap };
        }
        if !current_line.is_empty() {
            flex_lines.push(current_line);
        }

        // Pass 2: Distribute flex-grow and flex-shrink for each line
        let mut line_cross_sizes: Vec<f32> = Vec::new();

        for line in &flex_lines {
            let line_base_main_sum: f32 = line.iter().map(|&i| item_infos[i].base_main_size).sum();
            let line_gaps = if line.len() > 1 { gap * (line.len() - 1) as f32 } else { 0.0 };
            let line_total_hypothetical = line_base_main_sum + line_gaps;

            let free_space = container_main_limit - line_total_hypothetical;

            if free_space > 0.0 {
                let total_grow: f32 = line.iter().map(|&i| item_infos[i].flex_grow).sum();
                if total_grow > 0.0 {
                    for &i in line {
                        let extra = free_space * (item_infos[i].flex_grow / total_grow);
                        item_infos[i].final_main_size = item_infos[i].base_main_size + extra;
                    }
                }
            } else if free_space < 0.0 && flex_wrap == "nowrap" {
                let total_shrink_scaled: f32 = line.iter().map(|&i| item_infos[i].flex_shrink * item_infos[i].base_main_size).sum();
                if total_shrink_scaled > 0.0 {
                    for &i in line {
                        let shrink_amount = free_space.abs() * (item_infos[i].flex_shrink * item_infos[i].base_main_size / total_shrink_scaled);
                        item_infos[i].final_main_size = (item_infos[i].base_main_size - shrink_amount).max(0.0);
                    }
                }
            }

            let max_cross = line.iter().map(|&i| item_infos[i].measured_cross_size).fold(0.0, f32::max);
            line_cross_sizes.push(max_cross);
        }

        let total_lines_cross: f32 = line_cross_sizes.iter().sum::<f32>() + if flex_lines.len() > 1 { gap * (flex_lines.len() - 1) as f32 } else { 0.0 };
        let container_cross_limit = if explicit_h >= 0.0 { explicit_h } else { total_lines_cross };
        let cross_free_space = (container_cross_limit - total_lines_cross).max(0.0);

        let (cross_start_offset, cross_line_spacing) = match align_content {
            "center" => (cross_free_space / 2.0, gap),
            "flex-end" => (cross_free_space, gap),
            "space-between" => {
                let sp = if flex_lines.len() > 1 { cross_free_space / (flex_lines.len() - 1) as f32 } else { 0.0 };
                (0.0, gap + sp)
            }
            "space-around" => {
                let sp = if !flex_lines.is_empty() { cross_free_space / flex_lines.len() as f32 } else { 0.0 };
                (sp / 2.0, gap + sp)
            }
            _ => (0.0, gap),
        };

        let mut cur_cross_pos = if is_row { self.dimensions.content.y + cross_start_offset } else { self.dimensions.content.x + cross_start_offset };

        // Pass 3: Re-layout each child with definitive final_main_size
        for (line_idx, line) in flex_lines.iter().enumerate() {
            let line_gaps = if line.len() > 1 { gap * (line.len() - 1) as f32 } else { 0.0 };
            let line_final_main: f32 = line.iter().map(|&i| item_infos[i].final_main_size).sum::<f32>() + line_gaps;
            let line_free_main = (container_main_limit - line_final_main).max(0.0);

            let (start_offset, item_spacing) = match justify {
                "center" => (line_free_main / 2.0, gap),
                "flex-end" => (line_free_main, gap),
                "space-between" => {
                    let sp = if line.len() > 1 { line_free_main / (line.len() - 1) as f32 } else { 0.0 };
                    (0.0, gap + sp)
                }
                "space-around" => {
                    let sp = if !line.is_empty() { line_free_main / line.len() as f32 } else { 0.0 };
                    (sp / 2.0, gap + sp)
                }
                _ => (0.0, gap),
            };

            let line_max_cross = line_cross_sizes[line_idx];

            let mut cur_main_pos = if is_row {
                self.dimensions.content.x + start_offset
            } else {
                self.dimensions.content.y + start_offset
            };

            for &i in line {
                let info = &item_infos[i];
                let child = &mut self.children[info.child_idx];

                if is_row {
                    // Re-layout child with definitive final_main_size
                    let ch_h = child.layout(cur_main_pos, cur_cross_pos, info.final_main_size);
                    child.x = cur_main_pos;
                    child.dimensions.content.x = cur_main_pos;
                    child.width = info.final_main_size;
                    child.dimensions.content.width = info.final_main_size;

                    let cross_y = match align_items {
                        "center" => cur_cross_pos + (line_max_cross - ch_h) / 2.0,
                        "flex-end" => cur_cross_pos + (line_max_cross - ch_h),
                        "stretch" => {
                            if !child.style.contains_key("height") {
                                child.height = line_max_cross;
                                child.dimensions.content.height = line_max_cross;
                            }
                            cur_cross_pos
                        }
                        _ => cur_cross_pos,
                    };
                    child.y = cross_y;
                    child.dimensions.content.y = cross_y;

                    cur_main_pos += info.final_main_size + item_spacing;
                } else {
                    let ch_h = child.layout(cur_cross_pos, cur_main_pos, content_width);
                    child.y = cur_main_pos;
                    child.dimensions.content.y = cur_main_pos;
                    child.height = info.final_main_size;
                    child.dimensions.content.height = info.final_main_size;

                    let cross_x = match align_items {
                        "center" => cur_cross_pos + (content_width - ch_h) / 2.0,
                        "flex-end" => cur_cross_pos + (content_width - ch_h),
                        "stretch" => {
                            if !child.style.contains_key("width") {
                                child.width = content_width;
                                child.dimensions.content.width = content_width;
                            }
                            cur_cross_pos
                        }
                        _ => cur_cross_pos,
                    };
                    child.x = cross_x;
                    child.dimensions.content.x = cross_x;

                    cur_main_pos += info.final_main_size + item_spacing;
                }
            }

            cur_cross_pos += line_max_cross + cross_line_spacing;
        }

        let mut computed_h = if explicit_h >= 0.0 {
            explicit_h
        } else if is_row {
            total_lines_cross
        } else {
            container_main_limit
        };

        if let Some(min_h_str) = self.style.get("min-height") {
            let min_h = parse_length(min_h_str, 0.0, 0.0);
            computed_h = computed_h.max(min_h);
        }
        if let Some(max_h_str) = self.style.get("max-height") {
            let max_h = parse_length(max_h_str, 0.0, f32::MAX);
            computed_h = computed_h.min(max_h);
        }
        self.dimensions.content.height = computed_h;

        let border_box = self.dimensions.border_box();
        self.x = border_box.x;
        self.y = border_box.y;
        self.width = border_box.width;
        self.height = border_box.height;

        self.scroll_height = computed_h + self.dimensions.padding.top + self.dimensions.padding.bottom + self.dimensions.border.top + self.dimensions.border.bottom;
        self.scroll_width = content_width + self.dimensions.padding.left + self.dimensions.padding.right + self.dimensions.border.left + self.dimensions.border.right;
        let is_scroll_overflow = self.overflow == "auto" || self.overflow == "scroll" || self.overflow == "hidden";
        self.is_scroll_container = is_scroll_overflow && (self.scroll_height > self.height || self.scroll_width > self.width);

        if !self.transform.is_empty() {
            self.transform_matrix = resolve_transform_matrix(&self.transform, self.style.get("transform-origin"), self.width, self.height);
        }

        self.apply_position_offsets(x, y, max_width, self.dimensions.content.height);

        self.dimensions.margin_box().height
    }

    fn apply_position_offsets(&mut self, parent_x: f32, parent_y: f32, containing_w: f32, containing_h: f32) {
        let pos = self.style.get("position").map(|s| s.as_str()).unwrap_or("static");

        if pos == "relative" {
            let top = self.style.get("top").map(|v| parse_length(v, containing_h, 0.0)).unwrap_or(0.0);
            let bottom = self.style.get("bottom").map(|v| parse_length(v, containing_h, 0.0)).unwrap_or(0.0);
            let left = self.style.get("left").map(|v| parse_length(v, containing_w, 0.0)).unwrap_or(0.0);
            let right = self.style.get("right").map(|v| parse_length(v, containing_w, 0.0)).unwrap_or(0.0);

            let offset_x = if left != 0.0 { left } else { -right };
            let offset_y = if top != 0.0 { top } else { -bottom };

            self.x += offset_x;
            self.y += offset_y;
            self.dimensions.content.x += offset_x;
            self.dimensions.content.y += offset_y;
        } else if pos == "sticky" {
            let top_opt = self.style.get("top").map(|v| parse_length(v, containing_h, 0.0));
            if let Some(sticky_top) = top_opt {
                let clamped_top = self.y.max(parent_y + sticky_top);
                let max_y = (parent_y + containing_h - self.height).max(parent_y);
                let final_y = clamped_top.min(max_y);
                let diff_y = final_y - self.y;
                self.y = final_y;
                self.dimensions.content.y += diff_y;
            }
        } else if pos == "absolute" || pos == "fixed" {
            let top_opt = self.style.get("top").map(|v| parse_length(v, containing_h, 0.0));
            let left_opt = self.style.get("left").map(|v| parse_length(v, containing_w, 0.0));
            let right_opt = self.style.get("right").map(|v| parse_length(v, containing_w, 0.0));
            let bottom_opt = self.style.get("bottom").map(|v| parse_length(v, containing_h, 0.0));

            let base_x = if pos == "fixed" { 0.0 } else { parent_x };
            let base_y = if pos == "fixed" { 0.0 } else { parent_y };

            if let Some(left) = left_opt {
                self.x = base_x + left;
                self.dimensions.content.x = self.x;
            } else if let Some(right) = right_opt {
                self.x = base_x + containing_w - self.width - right;
                self.dimensions.content.x = self.x;
            }

            if let Some(top) = top_opt {
                self.y = base_y + top;
                self.dimensions.content.y = self.y;
            } else if let Some(bottom) = bottom_opt {
                self.y = base_y + containing_h - self.height - bottom;
                self.dimensions.content.y = self.y;
            }
        }
    }

    /// Accurate hit-testing traversing stacking contexts (highest z-index layer tested first),
    /// accounting for 2D CSS transforms (via inverse matrix point mapping) and nested scroll offsets
    pub fn hit_test(&self, target_x: f32, target_y: f32) -> Option<&LayoutBox> {
        let (local_x, local_y) = if let Some(matrix) = self.transform_matrix {
            if let Some(inv) = invert_transform(matrix) {
                transform_point(inv, target_x, target_y)
            } else {
                (target_x, target_y)
            }
        } else {
            (target_x, target_y)
        };

        let inside = local_x >= self.x && local_x <= self.x + self.width && local_y >= self.y && local_y <= self.y + self.height;
        if !inside && self.overflow == "hidden" {
            return None;
        }

        let inner_target_x = local_x + self.scroll_left;
        let inner_target_y = local_y + self.scroll_top;

        let mut sorted_children: Vec<&LayoutBox> = self.children.iter().collect();
        sorted_children.sort_by(|a, b| {
            let a_level = if a.z_index < 0 { 0 } else if !a.is_positioned && a.z_index == 0 { 1 } else if a.is_positioned && a.z_index == 0 { 2 } else { 3 };
            let b_level = if b.z_index < 0 { 0 } else if !b.is_positioned && b.z_index == 0 { 1 } else if b.is_positioned && b.z_index == 0 { 2 } else { 3 };
            if a_level != b_level { b_level.cmp(&a_level) } else { b.z_index.cmp(&a.z_index) }
        });

        for child in sorted_children {
            if let Some(hit) = child.hit_test(inner_target_x, inner_target_y) {
                return Some(hit);
            }
        }

        if inside {
            Some(self)
        } else {
            None
        }
    }

    /// Finds the deepest scroll container containing target point (x, y) that has scrollable content, accounting for transforms
    pub fn find_scroll_container_at_mut(&mut self, target_x: f32, target_y: f32) -> Option<&mut LayoutBox> {
        let (local_x, local_y) = if let Some(matrix) = self.transform_matrix {
            if let Some(inv) = invert_transform(matrix) {
                transform_point(inv, target_x, target_y)
            } else {
                (target_x, target_y)
            }
        } else {
            (target_x, target_y)
        };

        let inside = local_x >= self.x && local_x <= self.x + self.width && local_y >= self.y && local_y <= self.y + self.height;
        if !inside {
            return None;
        }

        let inner_x = local_x + self.scroll_left;
        let inner_y = local_y + self.scroll_top;

        for child in self.children.iter_mut().rev() {
            if let Some(sc) = child.find_scroll_container_at_mut(inner_x, inner_y) {
                return Some(sc);
            }
        }

        let max_scroll_y = (self.scroll_height - self.height).max(0.0);
        let max_scroll_x = (self.scroll_width - self.width).max(0.0);
        if (self.overflow == "auto" || self.overflow == "scroll") && (max_scroll_y > 0.0 || max_scroll_x > 0.0) {
            Some(self)
        } else {
            None
        }
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
    let node_id = node_borrow.node_id;

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
            let id = attributes.get("id").cloned().unwrap_or_default();
            let class_name = attributes.get("class").cloned().unwrap_or_default();
            let tag_name = tag.clone();

            if tag == "img" {
                let mut img_box = LayoutBox::new(BoxType::Image, style.clone(), href);
                img_box.id = id;
                img_box.class_name = class_name;
                img_box.tag_name = tag_name;
                img_box.node_id = node_id;
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
                input_box.id = id;
                input_box.class_name = class_name;
                input_box.tag_name = tag_name;
                input_box.node_id = node_id;
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
                btn_box.id = id;
                btn_box.class_name = class_name;
                btn_box.tag_name = tag_name;
                btn_box.node_id = node_id;
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

            let box_type = if display == "grid" || display == "inline-grid" {
                BoxType::Grid
            } else if display == "flex" {
                BoxType::Flex
            } else if display == "table" || tag == "table" {
                BoxType::Table
            } else if display == "table-row" || tag == "tr" {
                BoxType::TableRow
            } else if display == "table-cell" || tag == "td" || tag == "th" {
                BoxType::TableCell
            } else if display == "table-header-group" || display == "table-row-group" || display == "table-footer-group"
                || tag == "thead" || tag == "tbody" || tag == "tfoot"
            {
                BoxType::TableSection
            } else if display == "block" {
                BoxType::Block
            } else {
                BoxType::Inline
            };

            let mut root_box = LayoutBox::new(box_type, style.clone(), href);
            root_box.id = id;
            root_box.class_name = class_name;
            root_box.tag_name = tag_name;
            root_box.node_id = node_id;
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
            text_box.node_id = node_id;
            Some(text_box)
        }
    }
}
