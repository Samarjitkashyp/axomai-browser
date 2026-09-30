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
        }
    }

    pub fn specificity(&self) -> (usize, usize, usize) {
        match self {
            Selector::Universal => (0, 0, 0),
            Selector::ID(_) => (1, 0, 0),
            Selector::Class(_) | Selector::Attribute { .. } => (0, 1, 0),
            Selector::Tag(_) => (0, 0, 1),
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
            Selector::DirectChild(parent, child) | Selector::Descendant(parent, child) => {
                let (i1, c1, t1) = parent.specificity();
                let (i2, c2, t2) = child.specificity();
                (i1 + i2, c1 + c2, t1 + t2)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub selector: Selector,
    pub declarations: HashMap<String, String>,
}

pub struct CSSParser<'a> {
    css_text: &'a str,
}

impl<'a> CSSParser<'a> {
    pub fn new(css_text: &'a str) -> Self {
        CSSParser { css_text }
    }

    pub fn parse(&self) -> Vec<Rule> {
        let mut rules = Vec::new();
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

        rules
    }
}

pub fn parse_selector(sel_str: &str) -> Option<Selector> {
    let sel_str = sel_str.trim();
    if sel_str.is_empty() {
        return None;
    }

    let mut normalized = String::new();
    for ch in sel_str.chars() {
        if ch == '>' {
            normalized.push(' ');
            normalized.push('>');
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
        if tok == ">" {
            next_combinator = '>';
            continue;
        }

        if let Some(parsed) = parse_single_selector(tok) {
            match current_sel {
                None => {
                    current_sel = Some(parsed);
                }
                Some(prev) => {
                    if next_combinator == '>' {
                        current_sel = Some(Selector::DirectChild(Box::new(prev), Box::new(parsed)));
                    } else {
                        current_sel = Some(Selector::Descendant(Box::new(prev), Box::new(parsed)));
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

    if let Some(id_idx) = curr_part.find('#') {
        let id_part = curr_part[id_idx + 1..].to_string();
        sub_selectors.push(Selector::ID(id_part.to_lowercase()));
        curr_part = curr_part[..id_idx].to_string();
    }

    while let Some(cls_idx) = curr_part.find('.') {
        let class_part = curr_part[cls_idx + 1..].to_string();
        sub_selectors.push(Selector::Class(class_part.to_lowercase()));
        curr_part = curr_part[..cls_idx].to_string();
    }

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

pub fn parse_declarations(body_str: &str) -> HashMap<String, String> {
    let mut declarations = HashMap::new();
    for decl in body_str.split(';') {
        let decl = decl.trim();
        if let Some(colon_idx) = decl.find(':') {
            let prop = decl[..colon_idx].trim().to_lowercase();
            let val = decl[colon_idx + 1..].trim().to_lowercase();
            declarations.insert(prop, val);
        }
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
head, script, style { display: none; }
"#;

pub const INHERITED_PROPERTIES: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
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
            // Inherit from parent
            if let Some(ref pw) = parent_weak {
                if let Some(parent_rc) = pw.upgrade() {
                    let parent_borrow = parent_rc.borrow();
                    if let NodeType::Element {
                        style: ref parent_style,
                        ..
                    } = parent_borrow.node_type
                    {
                        for &prop in INHERITED_PROPERTIES {
                            if let Some(val) = parent_style.get(prop) {
                                style.insert(prop.to_string(), val.clone());
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
        }
    }

    let children = node.borrow().children.clone();
    for child in children {
        style_tree(&child, rules);
    }
}
