use std::collections::HashMap;

pub struct RustJSEngine {
    pub variables: HashMap<String, String>,
    pub console_logs: Vec<String>,
    pub dom_mutations: Vec<String>,
}

impl RustJSEngine {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            console_logs: Vec::new(),
            dom_mutations: Vec::new(),
        }
    }

    pub fn execute(&mut self, js_code: &str) -> bool {
        let mut dom_mutated = false;
        let statements: Vec<&str> = js_code.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        for stmt in statements {
            // 1. console.log(...)
            if stmt.starts_with("console.log") {
                if let Some(inner) = extract_args(stmt, "console.log") {
                    let val = self.eval_expr(&inner);
                    self.console_logs.push(val.clone());
                    println!("[Rust JS Console]: {}", val);
                }
                continue;
            }

            // 2. document.write(...)
            if stmt.starts_with("document.write") {
                if let Some(inner) = extract_args(stmt, "document.write") {
                    let html_snippet = self.eval_expr(&inner);
                    self.dom_mutations.push(html_snippet);
                    dom_mutated = true;
                }
                continue;
            }

            // 3. var / let / const assignment
            if stmt.starts_with("var ") || stmt.starts_with("let ") || stmt.starts_with("const ") {
                let rest = stmt.split_whitespace().skip(1).collect::<Vec<&str>>().join(" ");
                if let Some((var_name, expr)) = rest.split_once('=') {
                    let name = var_name.trim().to_string();
                    let val = self.eval_expr(expr.trim());
                    self.variables.insert(name, val);
                }
                continue;
            }
        }

        dom_mutated
    }

    pub fn eval_expr(&self, expr: &str) -> String {
        let expr = expr.trim();

        // Check for string concatenation '+' outside quotes
        if expr.contains('+') {
            let parts = split_addition(expr);
            if parts.len() > 1 {
                return parts.iter().map(|p| self.eval_expr(p)).collect::<Vec<String>>().join("");
            }
        }

        // String literal "..." or '...'
        if (expr.starts_with('"') && expr.ends_with('"')) || (expr.starts_with('\'') && expr.ends_with('\'')) {
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

fn extract_args(stmt: &str, prefix: &str) -> Option<String> {
    let rest = stmt.strip_prefix(prefix)?.trim();
    if rest.starts_with('(') && rest.ends_with(')') {
        Some(rest[1..rest.len() - 1].to_string())
    } else {
        None
    }
}

fn split_addition(expr: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut in_quote: Option<char> = None;
    let mut curr = String::new();

    for c in expr.chars() {
        if c == '"' || c == '\'' {
            if in_quote == Some(c) {
                in_quote = None;
            } else if in_quote.is_none() {
                in_quote = Some(c);
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
