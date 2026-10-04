//! On-device page assistant used by the toolbar "AI" button.
//! It is extractive (no model, no network): sentences are scored by term frequency, which is enough for a
//! useful summary, key topics and "ask this page" answers while keeping page text on the machine.

use std::collections::HashMap;

const STOP: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "any", "can", "had", "her", "was", "one", "our", "out",
    "has", "have", "this", "that", "with", "from", "they", "will", "would", "there", "their", "what", "about",
    "which", "when", "your", "been", "were", "more", "also", "than", "then", "them", "these", "those", "into",
    "over", "some", "such", "only", "other", "its", "his", "how", "who", "why", "where", "while", "does", "did",
    "just", "like", "very", "much", "many", "most", "each", "both", "being", "here", "after", "before", "because",
    "could", "should", "may", "might", "must", "shall", "said", "says", "per", "via", "get", "got", "use", "used",
];

fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.chars().count() > 2 && !STOP.contains(&w.as_str()))
        .collect()
}

pub fn sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, &ch) in chars.iter().enumerate() {
        cur.push(ch);
        // A full stop inside "static.hotjar.com" or "3.5" does not end a sentence; it must be followed by space/end.
        let boundary = match ch {
            '\n' | '।' => true,
            '.' | '!' | '?' => chars.get(i + 1).map_or(true, |n| n.is_whitespace()),
            _ => false,
        };
        if boundary {
            let s = cur.trim().to_string();
            if s.chars().count() >= 25 {
                out.push(s);
            }
            cur.clear();
        }
    }
    let s = cur.trim().to_string();
    if s.chars().count() >= 25 {
        out.push(s);
    }
    out
}

fn frequencies(text: &str) -> HashMap<String, f32> {
    let mut f: HashMap<String, f32> = HashMap::new();
    for t in tokens(text) {
        *f.entry(t).or_insert(0.0) += 1.0;
    }
    f
}

/// Top `n` sentences by summed term frequency, returned in their original order.
pub fn summarize(text: &str, n: usize) -> Vec<String> {
    let sents = sentences(text);
    if sents.len() <= n {
        return sents;
    }
    let freq = frequencies(text);
    let mut scored: Vec<(usize, f32)> = sents
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let toks = tokens(s);
            let len = toks.len().max(1) as f32;
            let sum: f32 = toks.iter().map(|t| freq.get(t).copied().unwrap_or(0.0)).sum();
            // Mild preference for earlier sentences, penalise very long ones.
            let position = 1.0 + 0.3 / (1.0 + i as f32 * 0.2);
            (i, sum / len.sqrt() * position)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut picked: Vec<usize> = scored.into_iter().take(n).map(|(i, _)| i).collect();
    picked.sort_unstable();
    picked.into_iter().map(|i| sents[i].clone()).collect()
}

pub fn keywords(text: &str, n: usize) -> Vec<String> {
    let mut f: Vec<(String, f32)> = frequencies(text).into_iter().filter(|(w, c)| w.chars().count() > 3 && *c >= 2.0).collect();
    f.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    f.into_iter().take(n).map(|(w, _)| w).collect()
}

/// Sentences that best match the question's terms.
pub fn answer(text: &str, question: &str, n: usize) -> Vec<String> {
    let q: Vec<String> = tokens(question);
    if q.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(usize, f32)> = sentences(text)
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let st = tokens(s);
            let hits = q.iter().filter(|t| st.contains(t)).count() as f32;
            (i, hits / (q.len() as f32) + 0.01 * hits)
        })
        .filter(|(_, sc)| *sc > 0.0)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let sents = sentences(text);
    scored.into_iter().take(n).map(|(i, _)| sents[i].clone()).collect()
}

pub struct Stats {
    pub words: usize,
    pub sentences: usize,
    pub minutes: usize,
}

pub fn stats(text: &str) -> Stats {
    let words = text.split_whitespace().count();
    Stats { words, sentences: sentences(text).len(), minutes: (words + 199) / 200 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "Assam is famous for its tea gardens and tea production. The Brahmaputra river flows through Assam. \
        Kaziranga National Park protects the one-horned rhinoceros in Assam. Tea from Assam is exported across the world. \
        The weather there is humid during summer months and rainfall is heavy.";

    #[test]
    fn summary_keeps_requested_count_in_order() {
        let s = summarize(TEXT, 2);
        assert_eq!(s.len(), 2);
        let first = TEXT.find(&s[0]).unwrap();
        let second = TEXT.find(&s[1]).unwrap();
        assert!(first < second);
    }

    #[test]
    fn keywords_prefer_repeated_terms() {
        let k = keywords(TEXT, 3);
        assert!(k.contains(&"assam".to_string()));
    }

    #[test]
    fn answer_finds_matching_sentence() {
        let a = answer(TEXT, "Which park protects rhinoceros?", 1);
        assert_eq!(a.len(), 1);
        assert!(a[0].contains("Kaziranga"));
    }

    #[test]
    fn dots_inside_words_do_not_split_sentences() {
        let s = sentences("Visit static.hotjar.com for the details of this plan. Version 3.5 is out now and it is fast.");
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("static.hotjar.com"));
    }

    #[test]
    fn empty_question_gives_nothing() {
        assert!(answer(TEXT, "the and", 2).is_empty());
    }

    #[test]
    fn stats_counts_words() {
        let s = stats("one two three four five");
        assert_eq!(s.words, 5);
        assert_eq!(s.minutes, 1);
    }
}
