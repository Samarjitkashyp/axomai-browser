//! Accessibility (a11y) & ARIA Tree Engine for Axomai Browser.
//! Converts DOM trees into Accessible Object Models (AOM) for screen readers and UI Automation.

use std::cell::RefCell;
use std::rc::Rc;
use crate::html_parser::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AriaRole {
    Generic,
    Button,
    Link,
    Heading { level: u8 },
    Alert,
    Dialog,
    Banner,
    Main,
    Navigation,
    Textbox,
    Checkbox { checked: bool },
    Radio { selected: bool },
    Table,
    Row,
    Cell,
}

#[derive(Debug, Clone)]
pub struct AccessibleNode {
    pub node_id: usize,
    pub role: AriaRole,
    pub name: String,
    pub description: Option<String>,
    pub is_focusable: bool,
    pub is_disabled: bool,
    pub children: Vec<AccessibleNode>,
}

pub struct AccessibilityTree {
    pub root: Option<AccessibleNode>,
}

impl AccessibilityTree {
    pub fn new() -> Self {
        AccessibilityTree { root: None }
    }

    /// Build accessibility tree from parsed DOM root node
    pub fn build(&mut self, dom_root: &Rc<RefCell<Node>>) {
        self.root = Some(self.build_node(dom_root));
    }

    fn build_node(&self, dom_node: &Rc<RefCell<Node>>) -> AccessibleNode {
        let n = dom_node.borrow();
        let tag = n.tag_name.as_deref().unwrap_or("");
        
        let role_attr = n.get_attribute("role");
        let aria_label = n.get_attribute("aria-label");
        let aria_hidden = n.get_attribute("aria-hidden").map(|v| v == "true").unwrap_or(false);

        let role = if let Some(r) = role_attr {
            match r.as_str() {
                "button" => AriaRole::Button,
                "link" => AriaRole::Link,
                "alert" => AriaRole::Alert,
                "dialog" => AriaRole::Dialog,
                "banner" => AriaRole::Banner,
                "main" => AriaRole::Main,
                "navigation" => AriaRole::Navigation,
                "textbox" => AriaRole::Textbox,
                _ => AriaRole::Generic,
            }
        } else {
            // Implicit HTML element roles
            match tag {
                "button" => AriaRole::Button,
                "a" => AriaRole::Link,
                "h1" => AriaRole::Heading { level: 1 },
                "h2" => AriaRole::Heading { level: 2 },
                "h3" => AriaRole::Heading { level: 3 },
                "input" => {
                    let input_type = n.get_attribute("type").unwrap_or_else(|| "text".to_string());
                    if input_type == "checkbox" {
                        AriaRole::Checkbox { checked: n.get_attribute("checked").is_some() }
                    } else if input_type == "radio" {
                        AriaRole::Radio { selected: n.get_attribute("checked").is_some() }
                    } else {
                        AriaRole::Textbox
                    }
                }
                "table" => AriaRole::Table,
                "tr" => AriaRole::Row,
                "td" | "th" => AriaRole::Cell,
                "nav" => AriaRole::Navigation,
                "main" => AriaRole::Main,
                "header" => AriaRole::Banner,
                _ => AriaRole::Generic,
            }
        };

        let mut accessible_name = aria_label.unwrap_or_default();
        if accessible_name.is_empty() {
            if let Some(ref text) = n.text_content {
                accessible_name = text.trim().to_string();
            }
        }

        let is_focusable = matches!(role, AriaRole::Button | AriaRole::Link | AriaRole::Textbox | AriaRole::Checkbox { .. } | AriaRole::Radio { .. });
        let is_disabled = n.get_attribute("disabled").is_some() || aria_hidden;

        let mut accessible_children = Vec::new();
        if !aria_hidden {
            for child in &n.children {
                accessible_children.push(self.build_node(child));
            }
        }

        AccessibleNode {
            node_id: n.node_id,
            role,
            name: accessible_name,
            description: n.get_attribute("aria-description"),
            is_focusable,
            is_disabled,
            children: accessible_children,
        }
    }
}
