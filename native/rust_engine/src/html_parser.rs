use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

pub type NodePtr = Rc<RefCell<NodeData>>;
pub type WeakNodePtr = Weak<RefCell<NodeData>>;

static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

pub fn alloc_node_id() -> usize {
    NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Document,
    DocumentType {
        name: String,
        public_id: Option<String>,
        system_id: Option<String>,
    },
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        style: HashMap<String, String>,
    },
    Text {
        text: String,
    },
    Comment {
        comment: String,
    },
}

#[derive(Debug)]
pub struct NodeData {
    pub node_id: usize,
    pub node_type: NodeType,
    pub parent: Option<WeakNodePtr>,
    pub children: Vec<NodePtr>,
}

impl NodeData {
    pub fn new_document() -> NodePtr {
        Rc::new(RefCell::new(NodeData {
            node_id: alloc_node_id(),
            node_type: NodeType::Document,
            parent: None,
            children: Vec::new(),
        }))
    }

    pub fn new_element(tag: &str, attributes: HashMap<String, String>) -> NodePtr {
        Rc::new(RefCell::new(NodeData {
            node_id: alloc_node_id(),
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
            node_id: alloc_node_id(),
            node_type: NodeType::Text {
                text: text.to_string(),
            },
            parent: None,
            children: Vec::new(),
        }))
    }

    pub fn new_comment(comment: &str) -> NodePtr {
        Rc::new(RefCell::new(NodeData {
            node_id: alloc_node_id(),
            node_type: NodeType::Comment {
                comment: comment.to_string(),
            },
            parent: None,
            children: Vec::new(),
        }))
    }

    pub fn add_child(parent: &NodePtr, child: &NodePtr) {
        child.borrow_mut().parent = Some(Rc::downgrade(parent));
        parent.borrow_mut().children.push(Rc::clone(child));
    }

    pub fn remove_child(parent: &NodePtr, child: &NodePtr) -> bool {
        let mut p = parent.borrow_mut();
        let init_len = p.children.len();
        p.children.retain(|c| !Rc::ptr_eq(c, child));
        if p.children.len() < init_len {
            child.borrow_mut().parent = None;
            true
        } else {
            false
        }
    }
}

pub const SELF_CLOSING_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

#[derive(Debug, Clone, PartialEq)]
pub enum HTMLToken {
    Doctype {
        name: Option<String>,
        public_id: Option<String>,
        system_id: Option<String>,
    },
    StartTag {
        name: String,
        attributes: HashMap<String, String>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Character(char),
    Text(String),
    Comment(String),
    EOF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenizerState {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    MarkupDeclarationOpen,
    CommentStart,
    Comment,
    CommentEnd,
    RawText,
}

pub struct HTMLTokenizer {
    input: Vec<char>,
    pos: usize,
    state: TokenizerState,
    raw_tag_name: String,
}

impl HTMLTokenizer {
    pub fn new(html: &str) -> Self {
        HTMLTokenizer {
            input: html.chars().collect(),
            pos: 0,
            state: TokenizerState::Data,
            raw_tag_name: String::new(),
        }
    }

    pub fn insert_input(&mut self, html_to_insert: &str) {
        let chars_to_insert: Vec<char> = html_to_insert.chars().collect();
        self.input.splice(self.pos..self.pos, chars_to_insert);
    }

    pub fn next_token(&mut self) -> HTMLToken {
        let mut current_tag_name = String::new();
        let mut current_attributes: HashMap<String, String> = HashMap::new();
        let mut current_attr_key = String::new();
        let mut current_attr_val = String::new();
        let mut is_self_closing = false;
        let mut comment_buf = String::new();

        while self.pos < self.input.len() {
            let c = self.input[self.pos];

            match self.state {
                TokenizerState::Data => {
                    if c == '<' {
                        self.state = TokenizerState::TagOpen;
                        self.pos += 1;
                    } else if c == '&' {
                        // Decode entity if present or emit character
                        let decoded = self.consume_entity();
                        return HTMLToken::Text(decoded);
                    } else {
                        self.pos += 1;
                        return HTMLToken::Character(c);
                    }
                }

                TokenizerState::TagOpen => {
                    if c == '!' {
                        self.state = TokenizerState::MarkupDeclarationOpen;
                        self.pos += 1;
                    } else if c == '/' {
                        self.state = TokenizerState::EndTagOpen;
                        self.pos += 1;
                    } else if c.is_ascii_alphabetic() {
                        current_tag_name.clear();
                        current_tag_name.push(c.to_ascii_lowercase());
                        self.state = TokenizerState::TagName;
                        self.pos += 1;
                    } else if c == '?' {
                        // Bogus comment
                        self.state = TokenizerState::Comment;
                        comment_buf.clear();
                        self.pos += 1;
                    } else {
                        self.state = TokenizerState::Data;
                        return HTMLToken::Character('<');
                    }
                }

                TokenizerState::EndTagOpen => {
                    if c.is_ascii_alphabetic() {
                        current_tag_name.clear();
                        current_tag_name.push(c.to_ascii_lowercase());
                        self.state = TokenizerState::TagName;
                        self.pos += 1;
                    } else if c == '>' {
                        self.state = TokenizerState::Data;
                        self.pos += 1;
                    } else {
                        self.state = TokenizerState::Comment;
                        comment_buf.clear();
                        self.pos += 1;
                    }
                }

                TokenizerState::TagName => {
                    if c.is_whitespace() {
                        self.state = TokenizerState::BeforeAttributeName;
                        self.pos += 1;
                    } else if c == '/' {
                        self.state = TokenizerState::SelfClosingStartTag;
                        self.pos += 1;
                    } else if c == '>' {
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        let tag = current_tag_name.to_lowercase();
                        if tag == "script" || tag == "style" || tag == "textarea" || tag == "title" {
                            self.state = TokenizerState::RawText;
                            self.raw_tag_name = tag.clone();
                        }
                        return HTMLToken::StartTag {
                            name: tag,
                            attributes: current_attributes,
                            self_closing: is_self_closing,
                        };
                    } else {
                        current_tag_name.push(c.to_ascii_lowercase());
                        self.pos += 1;
                    }
                }

                TokenizerState::BeforeAttributeName => {
                    if c.is_whitespace() {
                        self.pos += 1;
                    } else if c == '/' {
                        self.state = TokenizerState::SelfClosingStartTag;
                        self.pos += 1;
                    } else if c == '>' {
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        let tag = current_tag_name.to_lowercase();
                        if tag == "script" || tag == "style" || tag == "textarea" || tag == "title" {
                            self.state = TokenizerState::RawText;
                            self.raw_tag_name = tag.clone();
                        }
                        return HTMLToken::StartTag {
                            name: tag,
                            attributes: current_attributes,
                            self_closing: is_self_closing,
                        };
                    } else {
                        current_attr_key.clear();
                        current_attr_val.clear();
                        current_attr_key.push(c.to_ascii_lowercase());
                        self.state = TokenizerState::AttributeName;
                        self.pos += 1;
                    }
                }

                TokenizerState::AttributeName => {
                    if c.is_whitespace() {
                        self.state = TokenizerState::AfterAttributeName;
                        self.pos += 1;
                    } else if c == '=' {
                        self.state = TokenizerState::BeforeAttributeValue;
                        self.pos += 1;
                    } else if c == '/' {
                        current_attributes.insert(current_attr_key.clone(), String::new());
                        self.state = TokenizerState::SelfClosingStartTag;
                        self.pos += 1;
                    } else if c == '>' {
                        current_attributes.insert(current_attr_key.clone(), String::new());
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        let tag = current_tag_name.to_lowercase();
                        if tag == "script" || tag == "style" || tag == "textarea" || tag == "title" {
                            self.state = TokenizerState::RawText;
                            self.raw_tag_name = tag.clone();
                        }
                        return HTMLToken::StartTag {
                            name: tag,
                            attributes: current_attributes,
                            self_closing: is_self_closing,
                        };
                    } else {
                        current_attr_key.push(c.to_ascii_lowercase());
                        self.pos += 1;
                    }
                }

                TokenizerState::AfterAttributeName => {
                    if c.is_whitespace() {
                        self.pos += 1;
                    } else if c == '=' {
                        self.state = TokenizerState::BeforeAttributeValue;
                        self.pos += 1;
                    } else if c == '/' {
                        current_attributes.insert(current_attr_key.clone(), String::new());
                        self.state = TokenizerState::SelfClosingStartTag;
                        self.pos += 1;
                    } else if c == '>' {
                        current_attributes.insert(current_attr_key.clone(), String::new());
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        return HTMLToken::StartTag {
                            name: current_tag_name.to_lowercase(),
                            attributes: current_attributes,
                            self_closing: is_self_closing,
                        };
                    } else {
                        current_attributes.insert(current_attr_key.clone(), String::new());
                        current_attr_key.clear();
                        current_attr_key.push(c.to_ascii_lowercase());
                        self.state = TokenizerState::AttributeName;
                        self.pos += 1;
                    }
                }

                TokenizerState::BeforeAttributeValue => {
                    if c.is_whitespace() {
                        self.pos += 1;
                    } else if c == '"' {
                        self.state = TokenizerState::AttributeValueDoubleQuoted;
                        self.pos += 1;
                    } else if c == '\'' {
                        self.state = TokenizerState::AttributeValueSingleQuoted;
                        self.pos += 1;
                    } else if c == '>' {
                        current_attributes.insert(current_attr_key.clone(), String::new());
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        return HTMLToken::StartTag {
                            name: current_tag_name.to_lowercase(),
                            attributes: current_attributes,
                            self_closing: is_self_closing,
                        };
                    } else {
                        current_attr_val.clear();
                        current_attr_val.push(c);
                        self.state = TokenizerState::AttributeValueUnquoted;
                        self.pos += 1;
                    }
                }

                TokenizerState::AttributeValueDoubleQuoted => {
                    if c == '"' {
                        current_attributes.insert(current_attr_key.clone(), current_attr_val.clone());
                        self.state = TokenizerState::AfterAttributeValueQuoted;
                        self.pos += 1;
                    } else {
                        current_attr_val.push(c);
                        self.pos += 1;
                    }
                }

                TokenizerState::AttributeValueSingleQuoted => {
                    if c == '\'' {
                        current_attributes.insert(current_attr_key.clone(), current_attr_val.clone());
                        self.state = TokenizerState::AfterAttributeValueQuoted;
                        self.pos += 1;
                    } else {
                        current_attr_val.push(c);
                        self.pos += 1;
                    }
                }

                TokenizerState::AttributeValueUnquoted => {
                    if c.is_whitespace() {
                        current_attributes.insert(current_attr_key.clone(), current_attr_val.clone());
                        self.state = TokenizerState::BeforeAttributeName;
                        self.pos += 1;
                    } else if c == '>' {
                        current_attributes.insert(current_attr_key.clone(), current_attr_val.clone());
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        return HTMLToken::StartTag {
                            name: current_tag_name.to_lowercase(),
                            attributes: current_attributes,
                            self_closing: is_self_closing,
                        };
                    } else {
                        current_attr_val.push(c);
                        self.pos += 1;
                    }
                }

                TokenizerState::AfterAttributeValueQuoted => {
                    if c.is_whitespace() {
                        self.state = TokenizerState::BeforeAttributeName;
                        self.pos += 1;
                    } else if c == '/' {
                        self.state = TokenizerState::SelfClosingStartTag;
                        self.pos += 1;
                    } else if c == '>' {
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        return HTMLToken::StartTag {
                            name: current_tag_name.to_lowercase(),
                            attributes: current_attributes,
                            self_closing: is_self_closing,
                        };
                    } else {
                        self.state = TokenizerState::BeforeAttributeName;
                    }
                }

                TokenizerState::SelfClosingStartTag => {
                    if c == '>' {
                        is_self_closing = true;
                        self.pos += 1;
                        self.state = TokenizerState::Data;
                        return HTMLToken::StartTag {
                            name: current_tag_name.to_lowercase(),
                            attributes: current_attributes,
                            self_closing: true,
                        };
                    } else if c.is_whitespace() {
                        self.pos += 1;
                    } else {
                        self.state = TokenizerState::BeforeAttributeName;
                    }
                }

                TokenizerState::MarkupDeclarationOpen => {
                    let remaining: String = self.input[self.pos..].iter().take(7).collect();
                    let rem_lower = remaining.to_lowercase();
                    if rem_lower.starts_with("--") {
                        self.pos += 2;
                        comment_buf.clear();
                        self.state = TokenizerState::Comment;
                    } else if rem_lower.starts_with("doctype") {
                        self.pos += 7;
                        let mut doctype_str = String::new();
                        while self.pos < self.input.len() && self.input[self.pos] != '>' {
                            doctype_str.push(self.input[self.pos]);
                            self.pos += 1;
                        }
                        if self.pos < self.input.len() {
                            self.pos += 1; // consume '>'
                        }
                        self.state = TokenizerState::Data;
                        return HTMLToken::Doctype {
                            name: Some(doctype_str.trim().to_string()),
                            public_id: None,
                            system_id: None,
                        };
                    } else {
                        self.state = TokenizerState::Comment;
                    }
                }

                TokenizerState::CommentStart => {
                    self.state = TokenizerState::Comment;
                }

                TokenizerState::Comment => {
                    let rem: String = self.input[self.pos..].iter().take(3).collect();
                    if rem == "-->" {
                        self.pos += 3;
                        self.state = TokenizerState::Data;
                        return HTMLToken::Comment(comment_buf);
                    } else {
                        comment_buf.push(c);
                        self.pos += 1;
                    }
                }

                TokenizerState::CommentEnd => {
                    self.state = TokenizerState::Data;
                }

                TokenizerState::RawText => {
                    let close_tag = format!("</{}>", self.raw_tag_name);
                    let rem: String = self.input[self.pos..].iter().take(close_tag.len()).collect();
                    if rem.to_lowercase() == close_tag {
                        self.pos += close_tag.len();
                        self.state = TokenizerState::Data;
                        let tag = self.raw_tag_name.clone();
                        self.raw_tag_name.clear();
                        return HTMLToken::EndTag { name: tag };
                    } else {
                        self.pos += 1;
                        return HTMLToken::Character(c);
                    }
                }
            }
        }

        HTMLToken::EOF
    }

    fn consume_entity(&mut self) -> String {
        let mut entity_buf = String::new();
        self.pos += 1; // consume '&'
        while self.pos < self.input.len() && entity_buf.len() < 10 {
            let c = self.input[self.pos];
            if c == ';' {
                self.pos += 1;
                let full = format!("&{};", entity_buf);
                return html_escape::decode_html_entities(&full).to_string();
            } else if c.is_alphanumeric() || c == '#' {
                entity_buf.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = format!("&{}", entity_buf);
        html_escape::decode_html_entities(&raw).to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    AfterBody,
    AfterHtml,
}

pub struct HTMLParser<'a> {
    body: &'a str,
    open_elements: Vec<NodePtr>,
    root: Option<NodePtr>,
    head_element: Option<NodePtr>,
    mode: InsertionMode,
}

impl<'a> HTMLParser<'a> {
    pub fn new(body: &'a str) -> Self {
        let html_node = NodeData::new_element("html", HashMap::new());
        let body_node = NodeData::new_element("body", HashMap::new());
        NodeData::add_child(&html_node, &body_node);

        HTMLParser {
            body,
            open_elements: vec![Rc::clone(&html_node), Rc::clone(&body_node)],
            root: Some(Rc::clone(&html_node)),
            head_element: None,
            mode: InsertionMode::Initial,
        }
    }

    pub fn get_root(&self) -> NodePtr {
        if let Some(ref root) = self.root {
            Rc::clone(root)
        } else {
            let root = NodeData::new_element("html", HashMap::new());
            root
        }
    }

    pub fn parse(self) -> NodePtr {
        self.parse_interactive(|_, _, _| {})
    }

    pub fn parse_interactive<F>(mut self, mut on_script: F) -> NodePtr
    where
        F: FnMut(&HashMap<String, String>, &str, &mut HTMLTokenizer),
    {
        let mut tokenizer = HTMLTokenizer::new(self.body);
        let mut char_buffer = String::new();
        let mut active_script_attrs: Option<HashMap<String, String>> = None;
        let mut active_script_body = String::new();

        loop {
            let token = tokenizer.next_token();
            if token == HTMLToken::EOF {
                if !char_buffer.is_empty() {
                    self.insert_text(&char_buffer);
                    char_buffer.clear();
                }
                break;
            }

            match token {
                HTMLToken::Character(c) => {
                    if active_script_attrs.is_some() {
                        active_script_body.push(c);
                    } else {
                        char_buffer.push(c);
                    }
                }
                HTMLToken::Text(txt) => {
                    if active_script_attrs.is_some() {
                        active_script_body.push_str(&txt);
                    } else {
                        char_buffer.push_str(&txt);
                    }
                }
                HTMLToken::StartTag {
                    ref name,
                    ref attributes,
                    self_closing,
                } => {
                    if !char_buffer.is_empty() {
                        self.insert_text(&char_buffer);
                        char_buffer.clear();
                    }
                    if name == "script" && !self_closing {
                        active_script_attrs = Some(attributes.clone());
                        active_script_body.clear();
                    }
                    self.handle_start_tag(name, attributes.clone(), self_closing);
                }
                HTMLToken::EndTag { ref name } => {
                    if !char_buffer.is_empty() {
                        self.insert_text(&char_buffer);
                        char_buffer.clear();
                    }
                    if name == "script" {
                        if let Some(attrs) = active_script_attrs.take() {
                            let script_body = std::mem::take(&mut active_script_body);
                            if !script_body.is_empty() {
                                self.insert_text(&script_body);
                            }
                            on_script(&attrs, &script_body, &mut tokenizer);
                        }
                    }
                    self.handle_end_tag(name);
                }
                _ => {
                    if !char_buffer.is_empty() {
                        self.insert_text(&char_buffer);
                        char_buffer.clear();
                    }
                    self.handle_token(token);
                }
            }
        }

        self.finish()
    }

    fn handle_token(&mut self, token: HTMLToken) {
        match token {
            HTMLToken::Doctype { .. } => {
                self.mode = InsertionMode::BeforeHtml;
            }
            HTMLToken::StartTag {
                name,
                attributes,
                self_closing,
            } => {
                self.handle_start_tag(&name, attributes, self_closing);
            }
            HTMLToken::EndTag { name } => {
                self.handle_end_tag(&name);
            }
            HTMLToken::Comment(comment) => {
                let comment_node = NodeData::new_comment(&comment);
                if let Some(current) = self.current_node() {
                    NodeData::add_child(&current, &comment_node);
                }
            }
            _ => {}
        }
    }

    fn handle_start_tag(&mut self, tag_name: &str, attributes: HashMap<String, String>, self_closing: bool) {
        if tag_name == "html" {
            if let Some(ref root) = self.root {
                let mut b = root.borrow_mut();
                if let NodeType::Element { ref mut attributes: root_attrs, .. } = b.node_type {
                    root_attrs.extend(attributes);
                }
            }
            self.mode = InsertionMode::BeforeHead;
            return;
        }

        if tag_name == "head" {
            let head_node = NodeData::new_element("head", attributes);
            self.head_element = Some(Rc::clone(&head_node));
            if let Some(ref root) = self.root {
                NodeData::add_child(root, &head_node);
            }
            self.open_elements.push(head_node);
            self.mode = InsertionMode::InHead;
            return;
        }

        if tag_name == "body" {
            if let Some(ref root) = self.root {
                if let Some(existing_body) = find_body(root) {
                    let mut b = existing_body.borrow_mut();
                    if let NodeType::Element { ref mut attributes: body_attrs, .. } = b.node_type {
                        body_attrs.extend(attributes);
                    }
                    self.open_elements.push(existing_body);
                    self.mode = InsertionMode::InBody;
                    return;
                }
            }
            let body_node = NodeData::new_element("body", attributes);
            if let Some(ref root) = self.root {
                NodeData::add_child(root, &body_node);
            }
            self.open_elements.push(body_node);
            self.mode = InsertionMode::InBody;
            return;
        }

        let is_self_closing = self_closing || SELF_CLOSING_TAGS.contains(&tag_name);

        // Implicit <tbody> insertion when <tr>, <td>, or <th> is a direct child of <table>
        if tag_name == "tr" || tag_name == "td" || tag_name == "th" {
            let needs_implicit_tbody = self.current_node().map(|curr| {
                let b = curr.borrow();
                if let NodeType::Element { ref tag, .. } = b.node_type {
                    tag == "table"
                } else {
                    false
                }
            }).unwrap_or(false);

            if needs_implicit_tbody {
                let tbody = NodeData::new_element("tbody", HashMap::new());
                if let Some(current) = self.current_node() {
                    NodeData::add_child(&current, &tbody);
                }
                self.open_elements.push(tbody);
            }
        }

        let node = NodeData::new_element(tag_name, attributes);

        if let Some(current) = self.current_node() {
            NodeData::add_child(&current, &node);
        }

        if !is_self_closing {
            self.open_elements.push(node);
        }
    }

    fn handle_end_tag(&mut self, tag_name: &str) {
        if self.open_elements.len() > 1 {
            let mut match_idx = None;
            for idx in (0..self.open_elements.len()).rev() {
                let b = self.open_elements[idx].borrow();
                if let NodeType::Element { ref tag, .. } = b.node_type {
                    if tag == tag_name {
                        match_idx = Some(idx);
                        break;
                    }
                }
            }
            if let Some(idx) = match_idx {
                self.open_elements.truncate(idx);
            }
        }
    }

    fn insert_text(&mut self, text: &str) {
        let unescaped = html_escape::decode_html_entities(text).to_string();
        if unescaped.is_empty() {
            return;
        }
        if let Some(current) = self.current_node() {
            let text_node = NodeData::new_text(&unescaped);
            NodeData::add_child(&current, &text_node);
        }
    }

    fn current_node(&self) -> Option<NodePtr> {
        self.open_elements.last().map(Rc::clone).or_else(|| self.root.as_ref().map(Rc::clone))
    }

    fn finish(mut self) -> NodePtr {
        if let Some(root) = self.root {
            // Ensure <body> exists
            if find_body(&root).is_none() {
                let body = NodeData::new_element("body", HashMap::new());
                NodeData::add_child(&root, &body);
            }
            root
        } else {
            let root = NodeData::new_element("html", HashMap::new());
            let body = NodeData::new_element("body", HashMap::new());
            NodeData::add_child(&root, &body);
            root
        }
    }
}

pub fn find_element_by_id(node: &NodePtr, id: &str) -> Option<NodePtr> {
    let node_borrow = node.borrow();
    if let NodeType::Element { ref attributes, .. } = node_borrow.node_type {
        if let Some(elem_id) = attributes.get("id") {
            if elem_id == id {
                return Some(Rc::clone(node));
            }
        }
    }
    for child in &node_borrow.children {
        if let Some(found) = find_element_by_id(child, id) {
            return Some(found);
        }
    }
    None
}

pub fn query_selector(node: &NodePtr, selector: &str) -> Option<NodePtr> {
    let sel = selector.trim();
    if sel.is_empty() {
        return None;
    }

    let sub_selectors: Vec<&str> = sel.split(',').map(|s| s.trim()).collect();
    for sub in sub_selectors {
        if let Some(parsed) = crate::css_parser::parse_selector(sub) {
            if let Some(matched) = find_matching_node_selector(node, &parsed) {
                return Some(matched);
            }
        } else if let Some(matched) = fallback_query_selector(node, sub) {
            return Some(matched);
        }
    }
    None
}

pub fn query_selector_all(node: &NodePtr, selector: &str) -> Vec<NodePtr> {
    let sel = selector.trim();
    if sel.is_empty() {
        return Vec::new();
    }

    let sub_selectors: Vec<&str> = sel.split(',').map(|s| s.trim()).collect();
    let mut parsed_list = Vec::new();
    for sub in sub_selectors {
        if let Some(parsed) = crate::css_parser::parse_selector(sub) {
            parsed_list.push(parsed);
        }
    }

    let mut matches = Vec::new();
    if !parsed_list.is_empty() {
        collect_all_matching_nodes_selector(node, &parsed_list, &mut matches);
    } else {
        collect_matching_nodes(node, sel, &mut matches);
    }
    matches
}

fn find_matching_node_selector(node: &NodePtr, sel: &crate::css_parser::Selector) -> Option<NodePtr> {
    if sel.matches(node) {
        return Some(Rc::clone(node));
    }
    let node_borrow = node.borrow();
    for child in &node_borrow.children {
        if let Some(found) = find_matching_node_selector(child, sel) {
            return Some(found);
        }
    }
    None
}

fn collect_all_matching_nodes_selector(
    node: &NodePtr,
    selectors: &[crate::css_parser::Selector],
    matches: &mut Vec<NodePtr>,
) {
    if selectors.iter().any(|s| s.matches(node)) {
        matches.push(Rc::clone(node));
    }
    let node_borrow = node.borrow();
    for child in &node_borrow.children {
        collect_all_matching_nodes_selector(child, selectors, matches);
    }
}

fn fallback_query_selector(node: &NodePtr, sel: &str) -> Option<NodePtr> {
    if sel.starts_with('#') {
        return find_element_by_id(node, &sel[1..]);
    }
    let node_borrow = node.borrow();
    if let NodeType::Element { ref tag, ref attributes, .. } = node_borrow.node_type {
        if sel.starts_with('.') {
            let class_name = &sel[1..];
            if let Some(classes) = attributes.get("class") {
                if classes.split_whitespace().any(|c| c == class_name) {
                    return Some(Rc::clone(node));
                }
            }
        } else if tag == sel || sel == "*" {
            return Some(Rc::clone(node));
        }
    }
    for child in &node_borrow.children {
        if let Some(found) = fallback_query_selector(child, sel) {
            return Some(found);
        }
    }
    None
}

fn collect_matching_nodes(node: &NodePtr, sel: &str, matches: &mut Vec<NodePtr>) {
    let node_borrow = node.borrow();
    if let NodeType::Element { ref tag, ref attributes, .. } = node_borrow.node_type {
        if sel.starts_with('#') {
            let id = &sel[1..];
            if attributes.get("id").map(|s| s.as_str()) == Some(id) {
                matches.push(Rc::clone(node));
            }
        } else if sel.starts_with('.') {
            let class_name = &sel[1..];
            if let Some(classes) = attributes.get("class") {
                if classes.split_whitespace().any(|c| c == class_name) {
                    matches.push(Rc::clone(node));
                }
            }
        } else if tag == sel || sel == "*" {
            matches.push(Rc::clone(node));
        }
    }

    for child in &node_borrow.children {
        collect_matching_nodes(child, sel, matches);
    }
}


pub fn find_body(node: &NodePtr) -> Option<NodePtr> {
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

pub fn get_node_text_content(node: &NodePtr) -> String {
    let node_borrow = node.borrow();
    match &node_borrow.node_type {
        NodeType::Text { text } => text.clone(),
        NodeType::Element { .. } => {
            let mut out = String::new();
            for child in &node_borrow.children {
                out.push_str(&get_node_text_content(child));
            }
            out
        }
        _ => String::new(),
    }
}

pub fn set_node_text_content(node: &NodePtr, new_text: &str) {
    let mut node_mut = node.borrow_mut();
    node_mut.children.clear();
    let text_child = NodeData::new_text(new_text);
    text_child.borrow_mut().parent = Some(Rc::downgrade(node));
    node_mut.children.push(text_child);
}

pub fn set_node_inner_html(node: &NodePtr, html: &str) {
    let parsed_tree = HTMLParser::new(html).parse();
    let parsed_children = parsed_tree.borrow().children.clone();

    let mut node_mut = node.borrow_mut();
    node_mut.children.clear();

    for child in parsed_children {
        child.borrow_mut().parent = Some(Rc::downgrade(node));
        node_mut.children.push(child);
    }
}

pub fn remove_node(node: &NodePtr) -> bool {
    let parent_opt = {
        let b = node.borrow();
        b.parent.as_ref().and_then(|weak| weak.upgrade())
    };
    if let Some(parent) = parent_opt {
        NodeData::remove_child(&parent, node)
    } else {
        false
    }
}

pub fn get_node_inner_html(node: &NodePtr) -> String {
    let b = node.borrow();
    let mut out = String::new();
    for child in &b.children {
        serialize_node_html(child, &mut out);
    }
    out
}

fn serialize_node_html(node: &NodePtr, out: &mut String) {
    let b = node.borrow();
    match &b.node_type {
        NodeType::Text { text } => {
            out.push_str(&html_escape::encode_text(text));
        }
        NodeType::Element { tag, attributes, .. } => {
            out.push('<');
            out.push_str(tag);
            for (k, v) in attributes {
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                out.push_str(&html_escape::encode_double_quoted_attribute(v));
                out.push('"');
            }
            out.push('>');
            for child in &b.children {
                serialize_node_html(child, out);
            }
            if !SELF_CLOSING_TAGS.contains(&tag.as_str()) {
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
        }
        _ => {}
    }
}



