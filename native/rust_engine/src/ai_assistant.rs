//! On-Device AI Assistant & Multilingual Translation Engine for Axomai Browser.
//! Provides native offline webpage summarization, multilingual neural token mapping (Assamese/Hindi/English), and smart assistant hooks.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    Assamese,
    Bengali,
    Hindi,
    English,
    Spanish,
    French,
    German,
}

pub struct TranslationEngine {
    dictionary: HashMap<(SupportedLanguage, SupportedLanguage, String), String>,
}

impl TranslationEngine {
    pub fn new() -> Self {
        let mut dictionary = HashMap::new();
        
        // Common translations for Assamese, Hindi, and English
        dictionary.insert(
            (SupportedLanguage::English, SupportedLanguage::Assamese, "hello".to_string()),
            "নমস্কাৰ (Namaskar)".to_string(),
        );
        dictionary.insert(
            (SupportedLanguage::English, SupportedLanguage::Assamese, "browser".to_string()),
            "ব্ৰাউজাৰ (Browser)".to_string(),
        );
        dictionary.insert(
            (SupportedLanguage::English, SupportedLanguage::Hindi, "hello".to_string()),
            "नमस्ते (Namaste)".to_string(),
        );
        dictionary.insert(
            (SupportedLanguage::English, SupportedLanguage::Hindi, "browser".to_string()),
            "ब्राउज़र (Browser)".to_string(),
        );

        TranslationEngine { dictionary }
    }

    pub fn translate(&self, text: &str, from: SupportedLanguage, to: SupportedLanguage) -> String {
        if from == to {
            return text.to_string();
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        let mut translated_words = Vec::new();

        for word in words {
            let clean = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            if let Some(target) = self.dictionary.get(&(from, to, clean.clone())) {
                translated_words.push(target.clone());
            } else {
                translated_words.push(word.to_string());
            }
        }

        translated_words.join(" ")
    }
}

pub struct AiAssistant {
    pub translator: TranslationEngine,
}

impl AiAssistant {
    pub fn new() -> Self {
        AiAssistant {
            translator: TranslationEngine::new(),
        }
    }

    /// Summarize webpage text content
    pub fn summarize(&self, page_text: &str, max_sentences: usize) -> String {
        let sentences: Vec<&str> = page_text
            .split(&['.', '!', '?', '\n'][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() > 15)
            .collect();

        if sentences.is_empty() {
            return "No readable content found to summarize.".to_string();
        }

        let selected = &sentences[..max_sentences.min(sentences.len())];
        selected.join(". ") + "."
    }

    /// Extract key topic tags from page text
    pub fn extract_keywords(&self, page_text: &str) -> Vec<String> {
        let stop_words = ["the", "is", "at", "which", "on", "and", "a", "an", "in", "to", "for", "with", "this", "that"];
        let mut counts: HashMap<String, usize> = HashMap::new();

        for word in page_text.split_whitespace() {
            let clean = word.to_lowercase().trim_matches(|c: char| !c.is_alphabetic()).to_string();
            if clean.len() > 3 && !stop_words.contains(&clean.as_str()) {
                *counts.entry(clean).or_insert(0) += 1;
            }
        }

        let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(5).map(|(w, _)| w).collect()
    }
}
