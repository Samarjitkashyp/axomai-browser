use crate::html_parser::{HTMLParser, NodeData, NodePtr, NodeType};
use std::collections::HashMap;

pub struct RustJSEngine {
    pub variables: HashMap<String, String>,
    pub console_logs: Vec<String>,
    pub dom_mutations: Vec<String>,
}

impl RustJSEngine {
    pub fn new() -> Self {
        RustJSEngine {
            variables: HashMap::new(),
            console_logs: Vec::new(),
            dom_mutations: Vec::new(),
        }
    }

    pub fn execute(&mut self, js_code: &str, dom_root: Option<&NodePtr>) -> bool {
        let mut dom_mutated = false;
        let statements: Vec<&str> = js_code.split(';').map(|s| s.trim()).collect();

        for stmt in statements {
            if stmt.is_empty() {
                continue;
            }

            // 1. console.log(...)
            if stmt.starts_with("console.log") {
                if let (Some(open), Some(close)) = (stmt.find('('), stmt.rfind(')')) {
                    let expr = &stmt[open + 1..close];
                    let val = self.eval_expr(expr);
                    println!("[JS Console (Rust)]: {}", val);
                    self.console_logs.push(val);
                }
                continue;
            }

            // 2. document.write(...)
            if stmt.starts_with("document.write") {
                if let (Some(open), Some(close)) = (stmt.find('('), stmt.rfind(')')) {
                    let expr = &stmt[open + 1..close];
                    let html_snippet = self.eval_expr(expr);
                    self.dom_mutations.push(html_snippet.clone());

                    if let Some(root) = dom_root {
                        let snippet_node = HTMLParser::new(&html_snippet).parse();
                        let target = find_body(root).unwrap_or_else(|| Rc::clone(root));

                        let snippet_is_html = {
                            let b = snippet_node.borrow();
                            if let NodeType::Element { ref tag, .. } = b.node_type {
                                tag == "html"
                            } else {
                                false
                            }
                        };

                        if snippet_is_html {
                            let children = snippet_node.borrow().children.clone();
                            for child in children {
                                NodeData::add_child(&target, &child);
                            }
                        } else {
                            NodeData::add_child(&target, &snippet_node);
                        }
                    }

                    dom_mutated = true;
                }
                continue;
            }

            // 3. Variable declarations var x = ... / let / const
            if stmt.starts_with("var ") || stmt.starts_with("let ") || stmt.starts_with("const ") {
                let rest = stmt.split_whitespace().skip(1).collect::<Vec<&str>>().join(" ");
                if let Some(eq_idx) = rest.find('=') {
                    let var_name = rest[..eq_idx].trim().to_string();
                    let expr = rest[eq_idx + 1..].trim();
                    let val = self.eval_expr(expr);
                    self.variables.insert(var_name, val);
                }
                continue;
            }
        }

        dom_mutated
    }

    fn eval_expr(&self, expr_str: &str) -> String {
        let expr = expr_str.trim();

        // String concatenation with '+'
        if expr.contains('+') {
            let parts = split_addition(expr);
            if parts.len() > 1 {
                return parts.iter().map(|p| self.eval_expr(p)).collect::<String>();
            }
        }

        // String literal "..." or '...'
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            if expr.len() >= 2 {
                return expr[1..expr.len() - 1].to_string();
            }
        }

        // Variable lookup
        if let Some(val) = self.variables.get(expr) {
            return val.clone();
        }

        expr.to_string()
    }
}

use std::rc::Rc;

fn split_addition(expr: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut in_quote: Option<char> = None;
    let mut curr = String::new();

    for c in expr.chars() {
        if c == '"' || c == '\'' {
            if in_quote.is_none() {
                in_quote = Some(c);
            } else if in_quote == Some(c) {
                in_quote = None;
            }
            curr.push(c);
        } else if c == '+' && in_quote.is_none() {
            parts.push(curr.clone());
            curr.clear();
        } else {
            curr.push(c);
        }
    }
    if !curr.is_empty() {
        parts.push(curr);
    }
    parts
}

fn find_body(node: &NodePtr) -> Option<NodePtr> {
    let node_borrow = node.borrow();
    if let NodeType::Element { ref tag, .. } = node_borrow.node_type {
        if tag == "body" {
            return Some(Rc::clone(node));
        }
    }
    for child in &node_borrow.children {
        if let Some(body) = find_body(child) {
            return Some(body);
        }
    }
    None
}
