use crate::html_parser::{NodePtr, NodeType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Selector {
    Tag(String),
    Class(String),
    ID(String),
    Compound(Vec<Selector>),
    Descendant(Box<Selector>, Box<Selector>),
}

impl Selector {
    pub fn matches(&self, node: &NodePtr) -> bool {
        let node_borrow = node.borrow();
        match self {
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
            Selector::Compound(selectors) => selectors.iter().all(|s| s.matches(node)),
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

            if self.css_text[i..].starts_with("/*") {
                if let Some(end_comm) = self.css_text[i + 2..].find("*/") {
                    i += 2 + end_comm + 2;
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
            let sel_text = self.css_text[sel_start..i].trim();
            i += 1; // consume '{'

            let body_start = i;
            while i < n && chars[i] != '}' {
                i += 1;
            }
            let body_text = self.css_text[body_start..i].trim();
            if i < n {
                i += 1; // consume '}'
            }

            if sel_text.is_empty() {
                continue;
            }

            let declarations = parse_declarations(body_text);
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
    let parts: Vec<&str> = sel_str.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let selectors: Vec<Selector> = parts.into_iter().filter_map(parse_single_selector).collect();
    if selectors.is_empty() {
        return None;
    }

    if selectors.len() == 1 {
        return Some(selectors[0].clone());
    }

    let mut current = selectors[0].clone();
    for sel in selectors.into_iter().skip(1) {
        current = Selector::Descendant(Box::new(current), Box::new(sel));
    }
    Some(current)
}

fn parse_single_selector(part: &str) -> Option<Selector> {
    let mut sub_selectors = Vec::new();
    let mut curr_part = part.to_string();

    if let Some(id_idx) = curr_part.find('#') {
        let id_part = curr_part[id_idx + 1..].to_string();
        sub_selectors.push(Selector::ID(id_part.to_lowercase()));
        curr_part = curr_part[..id_idx].to_string();
    }

    if let Some(cls_idx) = curr_part.find('.') {
        let class_part = curr_part[cls_idx + 1..].to_string();
        sub_selectors.push(Selector::Class(class_part.to_lowercase()));
        curr_part = curr_part[..cls_idx].to_string();
    }

    if !curr_part.is_empty() {
        sub_selectors.push(Selector::Tag(curr_part.to_lowercase()));
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
    // 1. Gather matching rules first without holding mutable borrow
    let mut matched_declarations = HashMap::new();
    for rule in rules {
        if rule.selector.matches(node) {
            for (prop, val) in &rule.declarations {
                matched_declarations.insert(prop.clone(), val.clone());
            }
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
