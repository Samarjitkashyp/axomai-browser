use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Rule {
    pub selector: String,
    pub declarations: HashMap<String, String>,
}

pub struct CSSParser<'a> {
    css_text: &'a str,
}

impl<'a> CSSParser<'a> {
    pub fn new(css_text: &'a str) -> Self {
        Self { css_text }
    }

    pub fn parse(&self) -> Vec<Rule> {
        let mut rules = Vec::new();
        let chars: Vec<char> = self.css_text.chars().collect();
        let n = chars.len();
        let mut i = 0;

        while i < n {
            // Skip whitespace
            while i < n && chars[i].is_whitespace() {
                i += 1;
            }
            if i >= n {
                break;
            }

            // Skip comments /* ... */
            if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
                i += 2;
                while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            // Read selector up to '{'
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

            // Read body declarations up to '}'
            let body_start = i;
            while i < n && chars[i] != '}' {
                i += 1;
            }
            let body_text: String = chars[body_start..i].iter().collect();
            if i < n {
                i += 1; // consume '}'
            }

            if sel_text.is_empty() {
                continue;
            }

            let decls = parse_declarations(&body_text);
            if decls.is_empty() {
                continue;
            }

            // Handle comma-separated selectors (e.g. "head, script, style" or "h1, h2")
            for sub_sel in sel_text.split(',') {
                let sub_sel = sub_sel.trim();
                if !sub_sel.is_empty() {
                    rules.push(Rule {
                        selector: sub_sel.to_string(),
                        declarations: decls.clone(),
                    });
                }
            }
        }

        rules
    }
}

pub fn parse_declarations(body_text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for decl in body_text.split(';') {
        let decl = decl.trim();
        if let Some((prop, val)) = decl.split_once(':') {
            map.insert(prop.trim().to_lowercase(), val.trim().to_lowercase());
        }
    }
    map
}
