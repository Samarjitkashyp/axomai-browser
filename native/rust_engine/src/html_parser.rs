use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

pub type NodePtr = Rc<RefCell<NodeData>>;
pub type WeakNodePtr = Weak<RefCell<NodeData>>;

#[derive(Debug)]
pub enum NodeType {
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        style: HashMap<String, String>,
    },
    Text {
        text: String,
    },
}

#[derive(Debug)]
pub struct NodeData {
    pub node_type: NodeType,
    pub parent: Option<WeakNodePtr>,
    pub children: Vec<NodePtr>,
}

impl NodeData {
    pub fn new_element(tag: &str, attributes: HashMap<String, String>) -> NodePtr {
        Rc::new(RefCell::new(NodeData {
            node_type: NodeType::Element {
                tag: tag.to_lowercase(),
                attributes,
                style: HashMap::new(),
            },
            parent: None,
            children: Vec::new(),
        }))
    }

    pub fn new_text(text: &str) -> NodePtr {
        Rc::new(RefCell::new(NodeData {
            node_type: NodeType::Text {
                text: text.to_string(),
            },
            parent: None,
            children: Vec::new(),
        }))
    }

    pub fn add_child(parent: &NodePtr, child: &NodePtr) {
        child.borrow_mut().parent = Some(Rc::downgrade(parent));
        parent.borrow_mut().children.push(Rc::clone(child));
    }
}

pub const SELF_CLOSING_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

pub struct HTMLParser<'a> {
    body: &'a str,
    unfinished: Vec<NodePtr>,
}

impl<'a> HTMLParser<'a> {
    pub fn new(body: &'a str) -> Self {
        HTMLParser {
            body,
            unfinished: Vec::new(),
        }
    }

    pub fn parse(mut self) -> NodePtr {
        let mut text_buf = String::new();
        let chars: Vec<char> = self.body.chars().collect();
        let n = chars.len();
        let mut i = 0;

        while i < n {
            let c = chars[i];

            // 1. Comments
            if self.body[i..].starts_with("<!--") {
                if !text_buf.is_empty() {
                    self.add_text(&text_buf);
                    text_buf.clear();
                }
                if let Some(end_comment) = self.body[i + 4..].find("-->") {
                    i += 4 + end_comment + 3;
                } else {
                    i = n;
                }
                continue;
            }

            // 2. Script / Style raw contents
            if !self.unfinished.is_empty() {
                let top_tag = {
                    let top = self.unfinished.last().unwrap().borrow();
                    if let NodeType::Element { ref tag, .. } = top.node_type {
                        tag.clone()
                    } else {
                        String::new()
                    }
                };

                if top_tag == "script" || top_tag == "style" {
                    let close_tag = format!("</{}>", top_tag);
                    let body_slice = &self.body[i..];
                    let lower_slice = body_slice.to_lowercase();
                    if let Some(end_raw) = lower_slice.find(&close_tag) {
                        let raw_content = &self.body[i..i + end_raw];
                        if !raw_content.is_empty() {
                            self.add_text(raw_content);
                        }
                        i += end_raw + close_tag.len();
                        self.unfinished.pop();
                        continue;
                    } else {
                        let raw_content = &self.body[i..];
                        if !raw_content.is_empty() {
                            self.add_text(raw_content);
                        }
                        self.unfinished.pop();
                        break;
                    }
                }
            }

            if c == '<' {
                if !text_buf.is_empty() {
                    self.add_text(&text_buf);
                    text_buf.clear();
                }
            } else if c == '>' {
                self.add_tag(&text_buf);
                text_buf.clear();
            } else {
                text_buf.push(c);
            }

            i += 1;
        }

        if !text_buf.is_empty() {
            self.add_text(&text_buf);
        }

        self.finish()
    }

    fn add_text(&mut self, text: &str) {
        let unescaped = html_escape::decode_html_entities(text).to_string();
        if unescaped.is_empty() {
            return;
        }
        if let Some(top) = self.unfinished.last() {
            let text_node = NodeData::new_text(&unescaped);
            NodeData::add_child(top, &text_node);
        }
    }

    fn add_tag(&mut self, tag_content: &str) {
        let content = tag_content.trim();
        if content.is_empty() || content.starts_with('!') || content.starts_with('?') {
            return;
        }

        if content.starts_with('/') {
            let tag_name = content[1..].trim().to_lowercase();
            if self.unfinished.len() > 1 {
                let mut match_idx = None;
                for idx in (0..self.unfinished.len()).rev() {
                    let node = self.unfinished[idx].borrow();
                    if let NodeType::Element { ref tag, .. } = node.node_type {
                        if tag == &tag_name {
                            match_idx = Some(idx);
                            break;
                        }
                    }
                }
                if let Some(idx) = match_idx {
                    self.unfinished.truncate(idx);
                }
            }
            return;
        }

        let (tag_name, attributes) = self.parse_attributes(content);
        let is_self_closing = content.ends_with('/') || SELF_CLOSING_TAGS.contains(&tag_name.as_str());

        let node = NodeData::new_element(&tag_name, attributes);

        if is_self_closing {
            if let Some(top) = self.unfinished.last() {
                NodeData::add_child(top, &node);
            } else {
                self.unfinished.push(node);
            }
        } else {
            if let Some(top) = self.unfinished.last() {
                NodeData::add_child(top, &node);
            }
            self.unfinished.push(node);
        }
    }

    fn parse_attributes(&self, text: &str) -> (String, HashMap<String, String>) {
        let parts: Vec<&str> = text.splitn(2, char::is_whitespace).collect();
        let tag_name = parts[0].trim_matches('/').to_lowercase();
        let mut attributes = HashMap::new();

        if parts.len() < 2 {
            return (tag_name, attributes);
        }

        let attr_str = parts[1].trim_matches('/');
        let chars: Vec<char> = attr_str.chars().collect();
        let n = chars.len();
        let mut i = 0;

        while i < n {
            while i < n && chars[i].is_whitespace() {
                i += 1;
            }
            if i >= n {
                break;
            }

            let key_start = i;
            while i < n && !chars[i].is_whitespace() && chars[i] != '=' {
                i += 1;
            }
            let key: String = chars[key_start..i].iter().collect::<String>().to_lowercase();

            while i < n && chars[i].is_whitespace() {
                i += 1;
            }

            if i < n && chars[i] == '=' {
                i += 1;
                while i < n && chars[i].is_whitespace() {
                    i += 1;
                }
                if i < n {
                    if chars[i] == '"' || chars[i] == '\'' {
                        let quote = chars[i];
                        i += 1;
                        let val_start = i;
                        while i < n && chars[i] != quote {
                            i += 1;
                        }
                        let val: String = chars[val_start..i].iter().collect();
                        if i < n {
                            i += 1;
                        }
                        attributes.insert(key, val);
                    } else {
                        let val_start = i;
                        while i < n && !chars[i].is_whitespace() {
                            i += 1;
                        }
                        let val: String = chars[val_start..i].iter().collect();
                        attributes.insert(key, val);
                    }
                } else {
                    attributes.insert(key, String::new());
                }
            } else {
                attributes.insert(key, String::new());
            }
        }

        (tag_name, attributes)
    }

    fn finish(mut self) -> NodePtr {
        if !self.unfinished.is_empty() {
            return Rc::clone(&self.unfinished[0]);
        }
        let root = NodeData::new_element("html", HashMap::new());
        let body = NodeData::new_element("body", HashMap::new());
        NodeData::add_child(&root, &body);
        root
    }
}
