//! Reader Mode (Distraction-Free Article Extractor) for Axomai Browser.
//! Implements readability scoring algorithms to strip ads, navigation, and clutter, outputting clean typography.

use std::cell::RefCell;
use std::rc::Rc;
use crate::html_parser::Node;

#[derive(Debug, Clone)]
pub struct ReaderArticle {
    pub title: String,
    pub byline: Option<String>,
    pub content_html: String,
    pub text_content: String,
    pub excerpt: String,
    pub estimated_reading_time_mins: u32,
}

pub struct ReaderModeEngine;

impl ReaderModeEngine {
    /// Extract clean readable article content from a DOM tree
    pub fn parse(dom_root: &Rc<RefCell<Node>>, page_title: &str) -> Option<ReaderArticle> {
        let mut article_paragraphs = Vec::new();
        let mut full_text = String::new();

        Self::collect_article_paragraphs(dom_root, &mut article_paragraphs);

        if article_paragraphs.is_empty() {
            return None;
        }

        let mut content_html = String::new();
        for p in &article_paragraphs {
            content_html.push_str(&format!("<p>{}</p>\n", html_escape::encode_text(p)));
            full_text.push_str(p);
            full_text.push(' ');
        }

        let word_count = full_text.split_whitespace().count();
        let reading_time = (word_count as f32 / 200.0).ceil() as u32; // 200 words per minute average
        let excerpt = if full_text.len() > 150 {
            format!("{}...", &full_text[0..150].trim())
        } else {
            full_text.clone()
        };

        Some(ReaderArticle {
            title: page_title.to_string(),
            byline: None,
            content_html,
            text_content: full_text,
            excerpt,
            estimated_reading_time_mins: reading_time.max(1),
        })
    }

    fn collect_article_paragraphs(node: &Rc<RefCell<Node>>, paragraphs: &mut Vec<String>) {
        let n = node.borrow();
        let tag_owned = n.tag_name();
        let tag = tag_owned.as_deref().unwrap_or("");

        // Skip non-content tags
        if matches!(tag, "nav" | "footer" | "header" | "aside" | "script" | "style" | "noscript") {
            return;
        }

        if tag == "p" || tag == "article" {
            if let Some(ref text) = n.text_content() {
                let trimmed = text.trim();
                // Paragraphs with reasonable length are considered article body
                if trimmed.len() > 25 {
                    paragraphs.push(trimmed.to_string());
                }
            }
        }

        for child in &n.children {
            Self::collect_article_paragraphs(child, paragraphs);
        }
    }
}
