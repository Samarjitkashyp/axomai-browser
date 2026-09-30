use crate::html_parser::{NodePtr, NodeType};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeOp {
    Exists,
    Equals,
    StartsWith,
    EndsWith,
    Contains,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PseudoClassKind {
    FirstChild,
    LastChild,
    OnlyChild,
    NthChild { a: i32, b: i32 },
    NthOfType { a: i32, b: i32 },
    Disabled,
    Checked,
    Required,
    Root,
    Hover,
    Focus,
    Active,
}

#[derive(Debug, Clone)]
pub enum Selector {
    Universal,
    Tag(String),
    Class(String),
    ID(String),
    Attribute {
        name: String,
        op: AttributeOp,
        value: String,
    },
    Compound(Vec<Selector>),
    Descendant(Box<Selector>, Box<Selector>),
    DirectChild(Box<Selector>, Box<Selector>),
    AdjacentSibling(Box<Selector>, Box<Selector>),
    GeneralSibling(Box<Selector>, Box<Selector>),
    Not(Box<Selector>),
    PseudoClass(PseudoClassKind),
}

impl Selector {
    pub fn matches(&self, node: &NodePtr) -> bool {
        let node_borrow = node.borrow();
        match self {
            Selector::Universal => {
                matches!(node_borrow.node_type, NodeType::Element { .. })
            }
            Selector::Tag(tag_name) => {
                if let NodeType::Element { ref tag, .. } = node_borrow.node_type {
                    tag == tag_name
                } else {
                    false
                }
            }
            Selector::Class(class_name) => {
                if let NodeType::Element { ref attributes, .. } = node_borrow.node_type {
                    if let Some(cls_val) = attributes.get("class") {
                        cls_val.to_lowercase().split_whitespace().any(|c| c == class_name)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Selector::ID(id_name) => {
                if let NodeType::Element { ref attributes, .. } = node_borrow.node_type {
                    if let Some(id_val) = attributes.get("id") {
                        id_val.to_lowercase() == *id_name
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Selector::Attribute { name, op, value } => {
                if let NodeType::Element { ref attributes, .. } = node_borrow.node_type {
                    if let Some(attr_val) = attributes.get(name) {
                        let attr_lower = attr_val.to_lowercase();
                        let target_lower = value.to_lowercase();
                        match op {
                            AttributeOp::Exists => true,
                            AttributeOp::Equals => attr_lower == target_lower,
                            AttributeOp::StartsWith => attr_lower.starts_with(&target_lower),
                            AttributeOp::EndsWith => attr_lower.ends_with(&target_lower),
                            AttributeOp::Contains => attr_lower.contains(&target_lower),
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Selector::Compound(selectors) => selectors.iter().all(|s| s.matches(node)),
            Selector::DirectChild(parent_sel, child_sel) => {
                if !child_sel.matches(node) {
                    return false;
                }
                if let Some(parent_weak) = &node_borrow.parent {
                    if let Some(parent_rc) = parent_weak.upgrade() {
                        return parent_sel.matches(&parent_rc);
                    }
                }
                false
            }
            Selector::Descendant(ancestor_sel, descendant_sel) => {
                if !descendant_sel.matches(node) {
                    return false;
                }
                let mut curr = node_borrow.parent.clone();
                while let Some(parent_weak) = curr {
                    if let Some(parent_rc) = parent_weak.upgrade() {
                        if ancestor_sel.matches(&parent_rc) {
                            return true;
                        }
                        curr = parent_rc.borrow().parent.clone();
                    } else {
                        break;
                    }
                }
                false
            }
            Selector::AdjacentSibling(prev_sel, curr_sel) => {
                if !curr_sel.matches(node) {
                    return false;
                }
                if let Some(parent_weak) = &node_borrow.parent {
                    if let Some(parent_rc) = parent_weak.upgrade() {
                        let pb = parent_rc.borrow();
                        let elem_children: Vec<&NodePtr> = pb.children.iter().filter(|c| {
                            matches!(c.borrow().node_type, NodeType::Element { .. })
                        }).collect();
                        if let Some(idx) = elem_children.iter().position(|c| std::rc::Rc::ptr_eq(c, node)) {
                            if idx > 0 {
                                return prev_sel.matches(elem_children[idx - 1]);
                            }
                        }
                    }
                }
                false
            }
            Selector::GeneralSibling(prev_sel, curr_sel) => {
                if !curr_sel.matches(node) {
                    return false;
                }
                if let Some(parent_weak) = &node_borrow.parent {
                    if let Some(parent_rc) = parent_weak.upgrade() {
                        let pb = parent_rc.borrow();
                        let elem_children: Vec<&NodePtr> = pb.children.iter().filter(|c| {
                            matches!(c.borrow().node_type, NodeType::Element { .. })
                        }).collect();
                        if let Some(idx) = elem_children.iter().position(|c| std::rc::Rc::ptr_eq(c, node)) {
                            return elem_children[0..idx].iter().any(|prev_node| prev_sel.matches(prev_node));
                        }
                    }
                }
                false
            }
            Selector::Not(inner_sel) => !inner_sel.matches(node),
            Selector::PseudoClass(kind) => {
                if !matches!(node_borrow.node_type, NodeType::Element { .. }) {
                    return false;
                }
                match kind {
                    PseudoClassKind::Root => {
                        if let NodeType::Element { ref tag, .. } = node_borrow.node_type {
                            tag == "html" || node_borrow.parent.is_none()
                        } else {
                            false
                        }
                    }
                    PseudoClassKind::Disabled => {
                        if let NodeType::Element { ref attributes, .. } = node_borrow.node_type {
                            attributes.contains_key("disabled")
                        } else {
                            false
                        }
                    }
                    PseudoClassKind::Checked => {
                        if let NodeType::Element { ref attributes, .. } = node_borrow.node_type {
                            attributes.contains_key("checked") || attributes.contains_key("selected")
                        } else {
                            false
                        }
                    }
                    PseudoClassKind::Required => {
                        if let NodeType::Element { ref attributes, .. } = node_borrow.node_type {
                            attributes.contains_key("required")
                        } else {
                            false
                        }
                    }
                    PseudoClassKind::Hover | PseudoClassKind::Focus | PseudoClassKind::Active => false,
                    PseudoClassKind::FirstChild => {
                        if let Some(pw) = &node_borrow.parent {
                            if let Some(prc) = pw.upgrade() {
                                let pb = prc.borrow();
                                let first_elem = pb.children.iter().find(|c| matches!(c.borrow().node_type, NodeType::Element { .. }));
                                return first_elem.map(|c| std::rc::Rc::ptr_eq(c, node)).unwrap_or(false);
                            }
                        }
                        false
                    }
                    PseudoClassKind::LastChild => {
                        if let Some(pw) = &node_borrow.parent {
                            if let Some(prc) = pw.upgrade() {
                                let pb = prc.borrow();
                                let last_elem = pb.children.iter().rfind(|c| matches!(c.borrow().node_type, NodeType::Element { .. }));
                                return last_elem.map(|c| std::rc::Rc::ptr_eq(c, node)).unwrap_or(false);
                            }
                        }
                        false
                    }
                    PseudoClassKind::OnlyChild => {
                        if let Some(pw) = &node_borrow.parent {
                            if let Some(prc) = pw.upgrade() {
                                let pb = prc.borrow();
                                let elem_count = pb.children.iter().filter(|c| matches!(c.borrow().node_type, NodeType::Element { .. })).count();
                                return elem_count == 1;
                            }
                        }
                        false
                    }
                    PseudoClassKind::NthChild { a, b } => {
                        if let Some(pw) = &node_borrow.parent {
                            if let Some(prc) = pw.upgrade() {
                                let pb = prc.borrow();
                                let elem_children: Vec<&NodePtr> = pb.children.iter().filter(|c| {
                                    matches!(c.borrow().node_type, NodeType::Element { .. })
                                }).collect();
                                if let Some(idx) = elem_children.iter().position(|c| std::rc::Rc::ptr_eq(c, node)) {
                                    let k = (idx + 1) as i32;
                                    return match_nth(k, *a, *b);
                                }
                            }
                        }
                        false
                    }
                    PseudoClassKind::NthOfType { a, b } => {
                        let my_tag = if let NodeType::Element { ref tag, .. } = node_borrow.node_type {
                            tag.clone()
                        } else {
                            return false;
                        };
                        if let Some(pw) = &node_borrow.parent {
                            if let Some(prc) = pw.upgrade() {
                                let pb = prc.borrow();
                                let same_tag_children: Vec<&NodePtr> = pb.children.iter().filter(|c| {
                                    if let NodeType::Element { ref tag, .. } = c.borrow().node_type {
                                        tag == &my_tag
                                    } else {
                                        false
                                    }
                                }).collect();
                                if let Some(idx) = same_tag_children.iter().position(|c| std::rc::Rc::ptr_eq(c, node)) {
                                    let k = (idx + 1) as i32;
                                    return match_nth(k, *a, *b);
                                }
                            }
                        }
                        false
                    }
                }
            }
        }
    }

    pub fn specificity(&self) -> (usize, usize, usize) {
        match self {
            Selector::Universal => (0, 0, 0),
            Selector::ID(_) => (1, 0, 0),
            Selector::Class(_) | Selector::Attribute { .. } | Selector::PseudoClass(_) => (0, 1, 0),
            Selector::Tag(_) => (0, 0, 1),
            Selector::Not(inner) => inner.specificity(),
            Selector::Compound(list) => {
                let mut ids = 0;
                let mut classes = 0;
                let mut tags = 0;
                for s in list {
                    let (i, c, t) = s.specificity();
                    ids += i;
                    classes += c;
                    tags += t;
                }
                (ids, classes, tags)
            }
            Selector::DirectChild(parent, child)
            | Selector::Descendant(parent, child)
            | Selector::AdjacentSibling(parent, child)
            | Selector::GeneralSibling(parent, child) => {
                let (i1, c1, t1) = parent.specificity();
                let (i2, c2, t2) = child.specificity();
                (i1 + i2, c1 + c2, t1 + t2)
            }
        }
    }
}

fn match_nth(k: i32, a: i32, b: i32) -> bool {
    if a == 0 {
        k == b
    } else {
        let diff = k - b;
        (diff % a == 0) && (diff / a >= 0)
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub selector: Selector,
    pub declarations: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct KeyframeStep {
    pub offset: f32, // 0.0 (from / 0%) to 1.0 (to / 100%)
    pub declarations: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct KeyframeAnimation {
    pub name: String,
    pub steps: Vec<KeyframeStep>,
}

impl KeyframeAnimation {
    pub fn sample(&self, progress: f32) -> HashMap<String, String> {
        if self.steps.is_empty() {
            return HashMap::new();
        }
        if self.steps.len() == 1 {
            return self.steps[0].declarations.clone();
        }

        let p = progress.clamp(0.0, 1.0);
        let mut lower_idx = 0;
        let mut upper_idx = self.steps.len() - 1;

        for (i, step) in self.steps.iter().enumerate() {
            if step.offset <= p {
                lower_idx = i;
            }
            if step.offset >= p && i < upper_idx {
                upper_idx = i;
                break;
            }
        }

        let step_a = &self.steps[lower_idx];
        let step_b = &self.steps[upper_idx];

        if lower_idx == upper_idx || (step_b.offset - step_a.offset).abs() < 1e-5 {
            return step_a.declarations.clone();
        }

        let range = step_b.offset - step_a.offset;
        let local_p = ((p - step_a.offset) / range).clamp(0.0, 1.0);

        let mut result = step_a.declarations.clone();
        for (k, v_b) in &step_b.declarations {
            if let Some(v_a) = step_a.declarations.get(k) {
                let interp = interpolate_style_value(k, v_a, v_b, local_p);
                result.insert(k.clone(), interp);
            } else {
                result.insert(k.clone(), v_b.clone());
            }
        }
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimingFunction {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    StepStart,
    StepEnd,
    Steps(u32),
    CubicBezier(f32, f32, f32, f32),
}

impl TimingFunction {
    pub fn parse(s: &str) -> Self {
        let trimmed = s.trim().to_lowercase();
        if trimmed == "linear" {
            TimingFunction::Linear
        } else if trimmed == "ease" {
            TimingFunction::Ease
        } else if trimmed == "ease-in" {
            TimingFunction::EaseIn
        } else if trimmed == "ease-out" {
            TimingFunction::EaseOut
        } else if trimmed == "ease-in-out" {
            TimingFunction::EaseInOut
        } else if trimmed == "step-start" {
            TimingFunction::StepStart
        } else if trimmed == "step-end" {
            TimingFunction::StepEnd
        } else if trimmed.starts_with("steps(") && trimmed.ends_with(')') {
            let inner = &trimmed[6..trimmed.len() - 1];
            let n = inner.split(',').next().unwrap_or("1").trim().parse().unwrap_or(1);
            TimingFunction::Steps(n)
        } else if trimmed.starts_with("cubic-bezier(") && trimmed.ends_with(')') {
            let inner = &trimmed[13..trimmed.len() - 1];
            let parts: Vec<f32> = inner.split(',').filter_map(|p| p.trim().parse().ok()).collect();
            if parts.len() == 4 {
                TimingFunction::CubicBezier(parts[0], parts[1], parts[2], parts[3])
            } else {
                TimingFunction::Ease
            }
        } else {
            TimingFunction::Ease
        }
    }

    pub fn solve(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            TimingFunction::Linear => t,
            TimingFunction::Ease => solve_cubic_bezier(0.25, 0.1, 0.25, 1.0, t),
            TimingFunction::EaseIn => solve_cubic_bezier(0.42, 0.0, 1.0, 1.0, t),
            TimingFunction::EaseOut => solve_cubic_bezier(0.0, 0.0, 0.58, 1.0, t),
            TimingFunction::EaseInOut => solve_cubic_bezier(0.42, 0.0, 0.58, 1.0, t),
            TimingFunction::StepStart => if t > 0.0 { 1.0 } else { 0.0 },
            TimingFunction::StepEnd => if t >= 1.0 { 1.0 } else { 0.0 },
            TimingFunction::Steps(n) => {
                let n_f = (*n).max(1) as f32;
                (t * n_f).floor() / n_f
            }
            TimingFunction::CubicBezier(x1, y1, x2, y2) => solve_cubic_bezier(*x1, *y1, *x2, *y2, t),
        }
    }
}

pub fn solve_cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let mut t = x;
    for _ in 0..8 {
        let current_x = 3.0 * (1.0 - t) * (1.0 - t) * t * x1 + 3.0 * (1.0 - t) * t * t * x2 + t * t * t;
        let dx = current_x - x;
        if dx.abs() < 1e-4 {
            break;
        }
        let dxdt = 3.0 * (1.0 - t) * (1.0 - t) * x1 + 6.0 * (1.0 - t) * t * (x2 - x1) + 3.0 * t * t * (1.0 - x2);
        if dxdt.abs() < 1e-5 {
            break;
        }
        t -= dx / dxdt;
        t = t.clamp(0.0, 1.0);
    }
    3.0 * (1.0 - t) * (1.0 - t) * t * y1 + 3.0 * (1.0 - t) * t * t * y2 + t * t * t
}

#[derive(Debug, Clone)]
pub struct TransitionSpec {
    pub property: String,
    pub duration_sec: f32,
    pub timing_fn: TimingFunction,
    pub delay_sec: f32,
}

#[derive(Debug, Clone)]
pub struct AnimationSpec {
    pub name: String,
    pub duration_sec: f32,
    pub timing_fn: TimingFunction,
    pub delay_sec: f32,
    pub iteration_count: f32, // f32::INFINITY for "infinite"
    pub direction: String,     // "normal", "reverse", "alternate", "alternate-reverse"
    pub fill_mode: String,     // "none", "forwards", "backwards", "both"
}

pub struct StyleSheet {
    pub rules: Vec<Rule>,
    pub keyframes: HashMap<String, KeyframeAnimation>,
}

pub struct CSSParser<'a> {
    css_text: &'a str,
}

impl<'a> CSSParser<'a> {
    pub fn new(css_text: &'a str) -> Self {
        CSSParser { css_text }
    }

    pub fn parse(&self) -> Vec<Rule> {
        self.parse_stylesheet().rules
    }

    pub fn parse_stylesheet(&self) -> StyleSheet {
        let mut rules = Vec::new();
        let mut keyframes = HashMap::new();
        let chars: Vec<char> = self.css_text.chars().collect();
        let n = chars.len();
        let mut i = 0;

        while i < n {
            while i < n && chars[i].is_whitespace() {
                i += 1;
            }
            if i >= n {
                break;
            }

            // Block comment: /* ... */
            if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
                i += 2;
                while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < n {
                    i += 2;
                } else {
                    i = n;
                }
                continue;
            }

            let sel_start = i;
            while i < n && chars[i] != '{' {
                i += 1;
            }
            if i >= n {
                break;
            }
            let sel_text: String = chars[sel_start..i].iter().collect();
            let sel_text = sel_text.trim();
            i += 1; // consume '{'

            if sel_text.starts_with("@keyframes") || sel_text.starts_with("@-webkit-keyframes") {
                // Parse nested keyframe blocks until matching outer '}'
                let mut brace_depth = 1;
                let body_start = i;
                while i < n && brace_depth > 0 {
                    if chars[i] == '{' {
                        brace_depth += 1;
                    } else if chars[i] == '}' {
                        brace_depth -= 1;
                    }
                    if brace_depth > 0 {
                        i += 1;
                    }
                }
                let kf_body: String = chars[body_start..i].iter().collect();
                if i < n && chars[i] == '}' {
                    i += 1; // consume outer '}'
                }

                let anim_name = sel_text
                    .trim_start_matches("@keyframes")
                    .trim_start_matches("@-webkit-keyframes")
                    .trim();

                if !anim_name.is_empty() {
                    let kf_anim = parse_keyframe_blocks(anim_name, &kf_body);
                    keyframes.insert(anim_name.to_string(), kf_anim);
                }
                continue;
            }

            let body_start = i;
            while i < n && chars[i] != '}' {
                i += 1;
            }
            let body_text: String = chars[body_start..i].iter().collect();
            let body_text = body_text.trim();
            if i < n {
                i += 1; // consume '}'
            }

            if sel_text.is_empty() {
                continue;
            }

            let declarations = parse_declarations(&body_text);
            if declarations.is_empty() {
                continue;
            }

            for sub_sel in sel_text.split(',') {
                let sub_sel = sub_sel.trim();
                if !sub_sel.is_empty() {
                    if let Some(selector) = parse_selector(sub_sel) {
                        rules.push(Rule {
                            selector,
                            declarations: declarations.clone(),
                        });
                    }
                }
            }
        }

        StyleSheet { rules, keyframes }
    }
}

pub fn parse_selector(sel_str: &str) -> Option<Selector> {
    let sel_str = sel_str.trim();
    if sel_str.is_empty() {
        return None;
    }

    let mut normalized = String::new();
    let mut in_bracket = false;
    let mut in_paren = false;

    for ch in sel_str.chars() {
        if ch == '[' { in_bracket = true; }
        else if ch == ']' { in_bracket = false; }
        else if ch == '(' { in_paren = true; }
        else if ch == ')' { in_paren = false; }

        if !in_bracket && !in_paren && (ch == '>' || ch == '+' || ch == '~') {
            normalized.push(' ');
            normalized.push(ch);
            normalized.push(' ');
        } else {
            normalized.push(ch);
        }
    }

    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let mut current_sel: Option<Selector> = None;
    let mut next_combinator = ' ';

    for tok in tokens {
        if tok == ">" || tok == "+" || tok == "~" {
            next_combinator = tok.chars().next().unwrap();
            continue;
        }

        if let Some(parsed) = parse_single_selector(tok) {
            match current_sel {
                None => {
                    current_sel = Some(parsed);
                }
                Some(prev) => {
                    match next_combinator {
                        '>' => current_sel = Some(Selector::DirectChild(Box::new(prev), Box::new(parsed))),
                        '+' => current_sel = Some(Selector::AdjacentSibling(Box::new(prev), Box::new(parsed))),
                        '~' => current_sel = Some(Selector::GeneralSibling(Box::new(prev), Box::new(parsed))),
                        _ => current_sel = Some(Selector::Descendant(Box::new(prev), Box::new(parsed))),
                    }
                    next_combinator = ' ';
                }
            }
        }
    }

    current_sel
}

fn parse_single_selector(part: &str) -> Option<Selector> {
    let mut sub_selectors = Vec::new();
    let mut curr_part = part.to_string();

    // 1. Extract [attr=val]
    while let Some(start_bracket) = curr_part.find('[') {
        if let Some(end_bracket) = curr_part[start_bracket..].find(']') {
            let full_end = start_bracket + end_bracket;
            let attr_expr = &curr_part[start_bracket + 1..full_end];
            let attr_sel = if let Some(eq_idx) = attr_expr.find("^=") {
                let name = attr_expr[..eq_idx].trim().to_lowercase();
                let val = attr_expr[eq_idx + 2..].trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                Some(Selector::Attribute { name, op: AttributeOp::StartsWith, value: val })
            } else if let Some(eq_idx) = attr_expr.find("$=") {
                let name = attr_expr[..eq_idx].trim().to_lowercase();
                let val = attr_expr[eq_idx + 2..].trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                Some(Selector::Attribute { name, op: AttributeOp::EndsWith, value: val })
            } else if let Some(eq_idx) = attr_expr.find("*=") {
                let name = attr_expr[..eq_idx].trim().to_lowercase();
                let val = attr_expr[eq_idx + 2..].trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                Some(Selector::Attribute { name, op: AttributeOp::Contains, value: val })
            } else if let Some(eq_idx) = attr_expr.find('=') {
                let name = attr_expr[..eq_idx].trim().to_lowercase();
                let val = attr_expr[eq_idx + 1..].trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                Some(Selector::Attribute { name, op: AttributeOp::Equals, value: val })
            } else {
                let name = attr_expr.trim().to_lowercase();
                Some(Selector::Attribute { name, op: AttributeOp::Exists, value: String::new() })
            };

            if let Some(sel) = attr_sel {
                sub_selectors.push(sel);
            }
            curr_part = format!("{}{}", &curr_part[..start_bracket], &curr_part[full_end + 1..]);
        } else {
            break;
        }
    }

    // 2. Extract :not(...)
    while let Some(not_idx) = curr_part.find(":not(") {
        if let Some(end_paren) = curr_part[not_idx..].find(')') {
            let full_end = not_idx + end_paren;
            let inner = &curr_part[not_idx + 5..full_end];
            if let Some(inner_sel) = parse_single_selector(inner) {
                sub_selectors.push(Selector::Not(Box::new(inner_sel)));
            }
            curr_part = format!("{}{}", &curr_part[..not_idx], &curr_part[full_end + 1..]);
        } else {
            break;
        }
    }

    // 3. Extract pseudo-classes
    while let Some(colon_idx) = curr_part.find(':') {
        let pseudo_part = &curr_part[colon_idx..];
        let end_idx = pseudo_part[1..].find(|c: char| c == '.' || c == '#' || c == ':' || c == '[').map(|i| i + 1).unwrap_or(pseudo_part.len());
        let full_pseudo = &pseudo_part[..end_idx];

        let p_lower = full_pseudo.to_lowercase();
        let pseudo_sel = if p_lower == ":first-child" {
            Some(Selector::PseudoClass(PseudoClassKind::FirstChild))
        } else if p_lower == ":last-child" {
            Some(Selector::PseudoClass(PseudoClassKind::LastChild))
        } else if p_lower == ":only-child" {
            Some(Selector::PseudoClass(PseudoClassKind::OnlyChild))
        } else if p_lower == ":root" {
            Some(Selector::PseudoClass(PseudoClassKind::Root))
        } else if p_lower == ":disabled" {
            Some(Selector::PseudoClass(PseudoClassKind::Disabled))
        } else if p_lower == ":checked" {
            Some(Selector::PseudoClass(PseudoClassKind::Checked))
        } else if p_lower == ":required" {
            Some(Selector::PseudoClass(PseudoClassKind::Required))
        } else if p_lower == ":hover" {
            Some(Selector::PseudoClass(PseudoClassKind::Hover))
        } else if p_lower == ":focus" {
            Some(Selector::PseudoClass(PseudoClassKind::Focus))
        } else if p_lower == ":active" {
            Some(Selector::PseudoClass(PseudoClassKind::Active))
        } else if p_lower.starts_with(":nth-child(") && p_lower.ends_with(')') {
            let inner = &p_lower[11..p_lower.len() - 1];
            let (a, b) = parse_nth_expr(inner);
            Some(Selector::PseudoClass(PseudoClassKind::NthChild { a, b }))
        } else if p_lower.starts_with(":nth-of-type(") && p_lower.ends_with(')') {
            let inner = &p_lower[13..p_lower.len() - 1];
            let (a, b) = parse_nth_expr(inner);
            Some(Selector::PseudoClass(PseudoClassKind::NthOfType { a, b }))
        } else {
            None
        };

        if let Some(ps) = pseudo_sel {
            sub_selectors.push(ps);
        }
        curr_part = format!("{}{}", &curr_part[..colon_idx], &curr_part[colon_idx + end_idx..]);
    }

    // 4. Extract ID #
    if let Some(id_idx) = curr_part.find('#') {
        let id_part = curr_part[id_idx + 1..].to_string();
        sub_selectors.push(Selector::ID(id_part.to_lowercase()));
        curr_part = curr_part[..id_idx].to_string();
    }

    // 5. Extract Classes .
    while let Some(cls_idx) = curr_part.find('.') {
        let class_part = curr_part[cls_idx + 1..].to_string();
        sub_selectors.push(Selector::Class(class_part.to_lowercase()));
        curr_part = curr_part[..cls_idx].to_string();
    }

    // 6. Tag / Universal *
    if !curr_part.is_empty() {
        if curr_part == "*" {
            sub_selectors.push(Selector::Universal);
        } else {
            sub_selectors.push(Selector::Tag(curr_part.to_lowercase()));
        }
    }

    if sub_selectors.is_empty() {
        None
    } else if sub_selectors.len() == 1 {
        Some(sub_selectors[0].clone())
    } else {
        Some(Selector::Compound(sub_selectors))
    }
}

fn parse_nth_expr(s: &str) -> (i32, i32) {
    let s = s.trim().to_lowercase();
    if s == "odd" {
        (2, 1)
    } else if s == "even" {
        (2, 0)
    } else if let Ok(num) = s.parse::<i32>() {
        (0, num)
    } else if let Some(n_idx) = s.find('n') {
        let a_str = s[..n_idx].trim();
        let a = if a_str.is_empty() || a_str == "+" {
            1
        } else if a_str == "-" {
            -1
        } else {
            a_str.parse::<i32>().unwrap_or(1)
        };
        let b_str = s[n_idx + 1..].trim();
        let b = if b_str.is_empty() {
            0
        } else {
            b_str.replace(' ', "").parse::<i32>().unwrap_or(0)
        };
        (a, b)
    } else {
        (0, 1)
    }
}

pub fn parse_declarations(body_str: &str) -> HashMap<String, String> {
    let mut declarations = HashMap::new();
    let mut important_declarations = HashMap::new();

    for decl in body_str.split(';') {
        let decl = decl.trim();
        if let Some(colon_idx) = decl.find(':') {
            let prop = decl[..colon_idx].trim().to_lowercase();
            let mut val = decl[colon_idx + 1..].trim().to_string();
            let is_important = val.to_lowercase().ends_with("!important");
            if is_important {
                val = val[..val.len() - 10].trim().to_string();
                important_declarations.insert(prop.clone(), val.clone());
            }
            declarations.insert(prop, val);
        }
    }

    for (k, v) in important_declarations {
        declarations.insert(k, v);
    }
    declarations
}

pub const DEFAULT_UA_STYLES: &str = r#"
html { display: block; color: black; background-color: white; }
body { display: block; margin: 8px; font-size: 16px; font-family: sans-serif; }
div { display: block; }
p { display: block; margin-top: 10px; margin-bottom: 10px; }
h1 { display: block; font-size: 32px; font-weight: bold; margin-top: 15px; margin-bottom: 15px; }
h2 { display: block; font-size: 24px; font-weight: bold; margin-top: 12px; margin-bottom: 12px; }
h3 { display: block; font-size: 18px; font-weight: bold; margin-top: 10px; margin-bottom: 10px; }
b, strong { display: inline; font-weight: bold; }
i, em { display: inline; font-style: italic; }
a { display: inline; color: blue; text-decoration: underline; }
span { display: inline; }
img { display: inline-block; }
table { display: table; border-collapse: separate; border-spacing: 2px; }
thead { display: table-header-group; }
tbody { display: table-row-group; }
tfoot { display: table-footer-group; }
tr { display: table-row; }
td, th { display: table-cell; padding: 4px; vertical-align: inherit; }
th { font-weight: bold; text-align: center; }
caption { display: table-caption; text-align: center; }
head, script, style, template { display: none; }
"#;

pub const INHERITED_PROPERTIES: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
    "line-height",
    "letter-spacing",
    "text-align",
    "visibility",
    "cursor",
    "direction",
    "white-space",
];

pub fn style_tree(node: &NodePtr, rules: &[Rule]) {
    // 1. Gather matching rules sorted by specificity (ID > Class > Tag > Source Order)
    let mut matched_rules: Vec<(&Rule, (usize, usize, usize), usize)> = Vec::new();
    for (idx, rule) in rules.iter().enumerate() {
        if rule.selector.matches(node) {
            matched_rules.push((rule, rule.selector.specificity(), idx));
        }
    }
    // Sort ascending so higher specificity rules overwrite lower specificity declarations
    matched_rules.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2)));

    let mut matched_declarations = HashMap::new();
    for (rule, _, _) in matched_rules {
        for (prop, val) in &rule.declarations {
            matched_declarations.insert(prop.clone(), val.clone());
        }
    }

    let parent_weak = node.borrow().parent.clone();

    // 2. Now borrow node mutably to write computed styles
    {
        let mut node_borrow = node.borrow_mut();
        if let NodeType::Element {
            ref mut style,
            ref attributes,
            ..
        } = node_borrow.node_type
        {
            // Inherit from parent (including CSS Custom Properties --*)
            if let Some(ref pw) = parent_weak {
                if let Some(parent_rc) = pw.upgrade() {
                    let parent_borrow = parent_rc.borrow();
                    if let NodeType::Element {
                        style: ref parent_style,
                        ..
                    } = parent_borrow.node_type
                    {
                        for (k, v) in parent_style {
                            if k.starts_with("--") || INHERITED_PROPERTIES.contains(&k.as_str()) {
                                style.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }

            // Apply matched rules
            for (prop, val) in matched_declarations {
                style.insert(prop, val);
            }

            // Inline style="..."
            if let Some(inline_str) = attributes.get("style") {
                let inline_decls = parse_declarations(inline_str);
                for (prop, val) in inline_decls {
                    style.insert(prop, val);
                }
            }

            // 3. Resolve CSS Variables: var(--name, fallback)
            resolve_css_variables(style);

            // 4. Resolve Math Functions: calc(), min(), max(), clamp()
            resolve_css_math_functions(style);
        }
    }

    let children = node.borrow().children.clone();
    for child in children {
        style_tree(&child, rules);
    }
}

pub fn resolve_css_variables(style: &mut HashMap<String, String>) {
    let custom_props: HashMap<String, String> = style
        .iter()
        .filter(|(k, _)| k.starts_with("--"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    for (_k, v) in style.iter_mut() {
        if v.contains("var(--") {
            let mut resolved = v.clone();
            while let Some(var_start) = resolved.find("var(--") {
                if let Some(var_end) = resolved[var_start..].find(')') {
                    let full_end = var_start + var_end;
                    let inner = &resolved[var_start + 4..full_end];
                    let mut parts = inner.splitn(2, ',');
                    let var_name = parts.next().unwrap_or("").trim();
                    let fallback = parts.next().map(|s| s.trim()).unwrap_or("");

                    let replacement = custom_props
                        .get(var_name)
                        .map(|s| s.as_str())
                        .unwrap_or(fallback);

                    resolved = format!("{}{}{}", &resolved[..var_start], replacement, &resolved[full_end + 1..]);
                } else {
                    break;
                }
            }
            *v = resolved;
        }
    }
}

pub fn resolve_css_math_functions(style: &mut HashMap<String, String>) {
    for (_k, v) in style.iter_mut() {
        let v_trim = v.trim();
        if v_trim.starts_with("calc(") && v_trim.ends_with(')') {
            let inner = &v_trim[5..v_trim.len() - 1];
            if let Some(res) = eval_calc_expr(inner) {
                *v = format!("{:.2}px", res);
            }
        } else if v_trim.starts_with("min(") && v_trim.ends_with(')') {
            let inner = &v_trim[4..v_trim.len() - 1];
            let vals: Vec<f32> = inner.split(',').filter_map(|s| eval_calc_expr(s.trim())).collect();
            if let Some(&min_val) = vals.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) {
                *v = format!("{:.2}px", min_val);
            }
        } else if v_trim.starts_with("max(") && v_trim.ends_with(')') {
            let inner = &v_trim[4..v_trim.len() - 1];
            let vals: Vec<f32> = inner.split(',').filter_map(|s| eval_calc_expr(s.trim())).collect();
            if let Some(&max_val) = vals.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) {
                *v = format!("{:.2}px", max_val);
            }
        } else if v_trim.starts_with("clamp(") && v_trim.ends_with(')') {
            let inner = &v_trim[6..v_trim.len() - 1];
            let vals: Vec<f32> = inner.split(',').filter_map(|s| eval_calc_expr(s.trim())).collect();
            if vals.len() == 3 {
                let clamped = vals[1].max(vals[0]).min(vals[2]);
                *v = format!("{:.2}px", clamped);
            }
        }
    }
}

fn eval_calc_expr(expr: &str) -> Option<f32> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    // Split on + or - with spaces around them (CSS calc requires whitespace around + and -)
    if let Some(plus_idx) = expr.find(" + ") {
        let left = eval_calc_expr(&expr[..plus_idx])?;
        let right = eval_calc_expr(&expr[plus_idx + 3..])?;
        return Some(left + right);
    }
    if let Some(minus_idx) = expr.find(" - ") {
        let left = eval_calc_expr(&expr[..minus_idx])?;
        let right = eval_calc_expr(&expr[minus_idx + 3..])?;
        return Some(left - right);
    }
    if let Some(mul_idx) = expr.find(" * ") {
        let left = eval_calc_expr(&expr[..mul_idx])?;
        let right = eval_calc_expr(&expr[mul_idx + 3..])?;
        return Some(left * right);
    }
    if let Some(div_idx) = expr.find(" / ") {
        let left = eval_calc_expr(&expr[..div_idx])?;
        let right = eval_calc_expr(&expr[div_idx + 3..])?;
        if right != 0.0 {
            return Some(left / right);
        }
    }

    // Parse base unit (px, rem, em, pt, raw number)
    let s = expr.trim().trim_end_matches(';').to_lowercase();
    if s.ends_with("px") {
        s[..s.len() - 2].trim().parse::<f32>().ok()
    } else if s.ends_with("rem") {
        s[..s.len() - 3].trim().parse::<f32>().ok().map(|r| r * 16.0)
    } else if s.ends_with("em") {
        s[..s.len() - 2].trim().parse::<f32>().ok().map(|e| e * 16.0)
    } else if s.ends_with("pt") {
        s[..s.len() - 2].trim().parse::<f32>().ok().map(|pt| pt * 1.333)
    } else if s.ends_with('%') {
        // Approximate 100% to typical viewport / container default
        s[..s.len() - 1].trim().parse::<f32>().ok().map(|pct| pct * 8.0)
    } else {
        s.parse::<f32>().ok()
    }
}

    let children = node.borrow().children.clone();
    for child in children {
        style_tree(&child, rules);
    }
}

pub fn parse_keyframe_blocks(name: &str, body: &str) -> KeyframeAnimation {
    let mut steps = Vec::new();
    let chars: Vec<char> = body.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        while i < n && (chars[i].is_whitespace() || chars[i] == ';') {
            i += 1;
        }
        if i >= n {
            break;
        }

        let sel_start = i;
        while i < n && chars[i] != '{' {
            i += 1;
        }
        if i >= n {
            break;
        }
        let sel_str: String = chars[sel_start..i].iter().collect();
        let sel_str = sel_str.trim();
        i += 1; // consume '{'

        let body_start = i;
        while i < n && chars[i] != '}' {
            i += 1;
        }
        let step_body: String = chars[body_start..i].iter().collect();
        if i < n {
            i += 1; // consume '}'
        }

        let decls = parse_declarations(&step_body);
        for part in sel_str.split(',') {
            let part = part.trim();
            let offset_opt = if part == "from" || part == "0%" {
                Some(0.0)
            } else if part == "to" || part == "100%" {
                Some(1.0)
            } else if part.ends_with('%') {
                part[..part.len() - 1].trim().parse::<f32>().ok().map(|p| (p / 100.0).clamp(0.0, 1.0))
            } else {
                part.parse::<f32>().ok().map(|p| p.clamp(0.0, 1.0))
            };

            if let Some(offset) = offset_opt {
                steps.push(KeyframeStep {
                    offset,
                    declarations: decls.clone(),
                });
            }
        }
    }

    steps.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap_or(std::cmp::Ordering::Equal));
    KeyframeAnimation {
        name: name.to_string(),
        steps,
    }
}

pub fn parse_time_seconds(s: &str) -> f32 {
    let s = s.trim().to_lowercase();
    if s.ends_with("ms") {
        s[..s.len() - 2].trim().parse::<f32>().unwrap_or(0.0) / 1000.0
    } else if s.ends_with('s') {
        s[..s.len() - 1].trim().parse::<f32>().unwrap_or(0.0)
    } else {
        s.parse::<f32>().unwrap_or(0.0)
    }
}

pub fn parse_transition_shorthand(val: &str) -> Vec<TransitionSpec> {
    let mut specs = Vec::new();
    for part in val.split(',') {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let mut property = "all".to_string();
        let mut duration = 0.0;
        let mut timing_fn = TimingFunction::Ease;
        let mut delay = 0.0;
        let mut duration_found = false;

        for tok in tokens {
            let lower = tok.to_lowercase();
            if lower.ends_with("ms") || (lower.ends_with('s') && lower.chars().next().map(|c| c.is_ascii_digit() || c == '.').unwrap_or(false)) {
                let sec = parse_time_seconds(&lower);
                if !duration_found {
                    duration = sec;
                    duration_found = true;
                } else {
                    delay = sec;
                }
            } else if lower == "linear" || lower == "ease" || lower == "ease-in" || lower == "ease-out" || lower == "ease-in-out" || lower.starts_with("cubic-bezier") || lower.starts_with("steps") {
                timing_fn = TimingFunction::parse(&lower);
            } else {
                property = lower;
            }
        }

        specs.push(TransitionSpec {
            property,
            duration_sec: duration,
            timing_fn,
            delay_sec: delay,
        });
    }
    specs
}

pub fn parse_animation_shorthand(val: &str) -> Vec<AnimationSpec> {
    let mut specs = Vec::new();
    for part in val.split(',') {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let mut name = String::new();
        let mut duration = 0.0;
        let mut timing_fn = TimingFunction::Ease;
        let mut delay = 0.0;
        let mut iteration_count = 1.0;
        let mut direction = "normal".to_string();
        let mut fill_mode = "none".to_string();
        let mut duration_found = false;

        for tok in tokens {
            let lower = tok.to_lowercase();
            if lower.ends_with("ms") || (lower.ends_with('s') && lower.chars().next().map(|c| c.is_ascii_digit() || c == '.').unwrap_or(false)) {
                let sec = parse_time_seconds(&lower);
                if !duration_found {
                    duration = sec;
                    duration_found = true;
                } else {
                    delay = sec;
                }
            } else if lower == "infinite" {
                iteration_count = f32::INFINITY;
            } else if let Ok(num) = lower.parse::<f32>() {
                iteration_count = num;
            } else if lower == "linear" || lower == "ease" || lower == "ease-in" || lower == "ease-out" || lower == "ease-in-out" || lower.starts_with("cubic-bezier") || lower.starts_with("steps") {
                timing_fn = TimingFunction::parse(&lower);
            } else if lower == "normal" || lower == "reverse" || lower == "alternate" || lower == "alternate-reverse" {
                direction = lower;
            } else if lower == "forwards" || lower == "backwards" || lower == "both" || lower == "none" {
                fill_mode = lower;
            } else if name.is_empty() {
                name = lower;
            }
        }

        if !name.is_empty() {
            specs.push(AnimationSpec {
                name,
                duration_sec: duration,
                timing_fn,
                delay_sec: delay,
                iteration_count,
                direction,
                fill_mode,
            });
        }
    }
    specs
}

pub fn parse_color_rgba(s: &str) -> Option<(f32, f32, f32, f32)> {
    let s = s.trim().to_lowercase();
    if s == "transparent" {
        return Some((0.0, 0.0, 0.0, 0.0));
    }
    if s == "black" { return Some((0.0, 0.0, 0.0, 1.0)); }
    if s == "white" { return Some((255.0, 255.0, 255.0, 1.0)); }
    if s == "red" { return Some((255.0, 0.0, 0.0, 1.0)); }
    if s == "green" { return Some((0.0, 128.0, 0.0, 1.0)); }
    if s == "blue" { return Some((0.0, 0.0, 255.0, 1.0)); }
    if s == "yellow" { return Some((255.0, 255.0, 0.0, 1.0)); }

    if s.starts_with('#') {
        let hex = &s[1..];
        if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()? as f32;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()? as f32;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()? as f32;
            return Some((r, g, b, 1.0));
        } else if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32;
            return Some((r, g, b, 1.0));
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32;
            let a = (u8::from_str_radix(&hex[6..8], 16).ok()? as f32) / 255.0;
            return Some((r, g, b, a));
        }
    } else if s.starts_with("rgb(") && s.ends_with(')') {
        let inner = &s[4..s.len() - 1];
        let parts: Vec<f32> = inner.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        if parts.len() >= 3 {
            return Some((parts[0], parts[1], parts[2], 1.0));
        }
    } else if s.starts_with("rgba(") && s.ends_with(')') {
        let inner = &s[5..s.len() - 1];
        let parts: Vec<f32> = inner.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        if parts.len() >= 4 {
            return Some((parts[0], parts[1], parts[2], parts[3]));
        }
    }
    None
}

pub fn interpolate_style_value(prop: &str, start_val: &str, end_val: &str, progress: f32) -> String {
    let p = progress.clamp(0.0, 1.0);
    let s_trim = start_val.trim();
    let e_trim = end_val.trim();

    // 1. Color properties
    if prop.contains("color") || prop == "fill" || prop == "stroke" {
        if let (Some(c1), Some(c2)) = (parse_color_rgba(s_trim), parse_color_rgba(e_trim)) {
            let r = (c1.0 + (c2.0 - c1.0) * p).round().clamp(0.0, 255.0);
            let g = (c1.1 + (c2.1 - c1.1) * p).round().clamp(0.0, 255.0);
            let b = (c1.2 + (c2.2 - c1.2) * p).round().clamp(0.0, 255.0);
            let a = (c1.3 + (c2.3 - c1.3) * p).clamp(0.0, 1.0);
            if (a - 1.0).abs() < 1e-3 {
                return format!("rgb({}, {}, {})", r as u32, g as u32, b as u32);
            } else {
                return format!("rgba({}, {}, {}, {:.3})", r as u32, g as u32, b as u32, a);
            }
        }
    }

    // 2. Opacity
    if prop == "opacity" {
        let o1 = s_trim.parse::<f32>().unwrap_or(1.0);
        let o2 = e_trim.parse::<f32>().unwrap_or(1.0);
        return format!("{:.4}", o1 + (o2 - o1) * p);
    }

    // 3. Lengths with units (e.g. px, %, em, rem)
    if let (Some((v1, u1)), Some((v2, u2))) = (extract_num_unit(s_trim), extract_num_unit(e_trim)) {
        if u1 == u2 {
            let v = v1 + (v2 - v1) * p;
            return format!("{:.2}{}", v, u1);
        }
    }

    // 4. Raw numbers / z-index
    if let (Ok(n1), Ok(n2)) = (s_trim.parse::<f32>(), e_trim.parse::<f32>()) {
        let v = n1 + (n2 - n1) * p;
        return format!("{:.2}", v);
    }

    // Fallback: discrete step at 50%
    if p < 0.5 {
        start_val.to_string()
    } else {
        end_val.to_string()
    }
}

fn extract_num_unit(s: &str) -> Option<(f32, &str)> {
    let s = s.trim();
    let num_end = s.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(s.len());
    if num_end == 0 {
        return None;
    }
    let num = s[..num_end].parse::<f32>().ok()?;
    let unit = &s[num_end..];
    Some((num, unit))
}

