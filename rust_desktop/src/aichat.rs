//! Axom AI chat. The question (and, if the user leaves it on, the text of the page) goes to Axomai's own chat server,
//! which holds the OpenAI key and the model; nothing about either is stored in or settable from the browser.
//! The answer streams back and is shown in the chat box of the AI popup.

use crate::app::App;
use crate::web::WebEvent;
use serde_json::{json, Value};

pub const CHAT_URL: &str = "https://axomai-browser.aiaxom.co.in/v1/chat";
const MAX_MESSAGES: usize = 20;
const MAX_MESSAGE_CHARS: usize = 4000;
const MAX_PAGE_CHARS: usize = 12000;

/// `AXOMAI_CHAT_URL` replaces the server address (used to test against a local pretend server).
pub fn chat_url() -> String {
    std::env::var("AXOMAI_CHAT_URL").ok().filter(|u| !u.trim().is_empty()).unwrap_or_else(|| CHAT_URL.to_string())
}

fn clip(s: &str, n: usize) -> String {
    s.chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').take(n).collect()
}

/// What the popup sends, cleaned up: only user / assistant messages, bounded, and the page as plain text.
pub fn sanitize(v: &Value) -> Option<Value> {
    let msgs = v.get("messages")?.as_array()?;
    if msgs.is_empty() {
        return None;
    }
    let start = msgs.len().saturating_sub(MAX_MESSAGES);
    let mut out = Vec::new();
    for m in &msgs[start..] {
        let role = m.get("role")?.as_str()?;
        if role != "user" && role != "assistant" {
            return None;
        }
        let content = clip(m.get("content")?.as_str()?, MAX_MESSAGE_CHARS);
        if content.trim().is_empty() {
            return None;
        }
        out.push(json!({"role": role, "content": content}));
    }
    if out.last()?.get("role")?.as_str()? != "user" {
        return None;
    }
    let mut body = json!({"messages": out});
    if let Some(p) = v.get("page").filter(|p| p.is_object()) {
        let text = clip(p.get("text").and_then(|t| t.as_str()).unwrap_or(""), MAX_PAGE_CHARS);
        if !text.trim().is_empty() {
            let url = p.get("url").and_then(|u| u.as_str()).unwrap_or("");
            let url = if url.starts_with("http://") || url.starts_with("https://") { clip(url, 300) } else { String::new() };
            body["page"] = json!({"url": url, "title": clip(p.get("title").and_then(|t| t.as_str()).unwrap_or(""), 200), "text": text});
        }
    }
    Some(body)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Chunk {
    Text(String),
    Done,
    Error(String),
}

/// One line of the server's event stream (`data: {"t":"..."}`).
pub fn parse_line(line: &str) -> Option<Chunk> {
    let data = line.trim().strip_prefix("data:")?.trim();
    let v: Value = serde_json::from_str(data).ok()?;
    if let Some(t) = v.get("t").and_then(|t| t.as_str()) {
        return Some(Chunk::Text(t.to_string()));
    }
    if v.get("done").and_then(|d| d.as_bool()) == Some(true) {
        return Some(Chunk::Done);
    }
    v.get("message").and_then(|m| m.as_str()).map(|m| Chunk::Error(m.to_string()))
}

/// A random id for this installation; it only lets the server count the daily limit, it says nothing about the person.
pub fn new_client_id() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::getrandom(&mut b);
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

fn stream_request(shared: crate::web::WebShared, rid: String, body: Value, client: String) {
    use std::io::{BufRead, BufReader};
    let send = |kind: &str, text: String| shared.push_event(WebEvent::PageData("aichat".into(), format!("{}:{}", kind, rid), text));
    let result = ureq::post(&chat_url())
        .set("Content-Type", "application/json")
        .set("X-Axomai-Client", &client)
        .set("X-Axomai-App", &format!("AxomaiBrowser/{}", crate::update::current_version()))
        .timeout(std::time::Duration::from_secs(90))
        .send_string(&body.to_string());
    match result {
        Ok(resp) => {
            let mut finished = false;
            for line in BufReader::new(resp.into_reader()).lines().map_while(Result::ok) {
                match parse_line(&line) {
                    Some(Chunk::Text(t)) => send("chunk", t),
                    Some(Chunk::Done) => {
                        finished = true;
                        send("done", String::new());
                    }
                    Some(Chunk::Error(m)) => {
                        finished = true;
                        send("error", m);
                    }
                    None => {}
                }
            }
            if !finished {
                send("error", "The connection to Axom AI was cut. Please try again.".into());
            }
        }
        Err(ureq::Error::Status(_, resp)) => {
            let msg = resp.into_string().ok().and_then(|b| serde_json::from_str::<Value>(&b).ok()).and_then(|v| v.get("message").and_then(|m| m.as_str()).map(String::from));
            send("error", msg.unwrap_or_else(|| "Axom AI is not available right now. Please try again later.".into()));
        }
        Err(_) => send("error", "Could not reach Axom AI. Check your internet connection.".into()),
    }
}

impl App {
    fn ai_client_id(&mut self) -> String {
        if let Some(st) = &self.storage {
            if let Some(id) = st.get_setting("ai_client_id").ok().flatten().filter(|i| i.len() >= 16) {
                return id;
            }
            let id = new_client_id();
            let _ = st.set_setting("ai_client_id", &id);
            return id;
        }
        new_client_id()
    }

    /// `ai-chat/<request id>/<url-encoded JSON>` from the AI popup.
    pub fn ai_chat_send(&mut self, rid: &str, raw_json: &str) {
        let Some(body) = serde_json::from_str::<Value>(raw_json).ok().as_ref().and_then(sanitize) else {
            return self.ai_chat_push(rid, "error", "That message could not be sent.");
        };
        if rid.is_empty() || rid.len() > 20 || !rid.chars().all(|c| c.is_ascii_alphanumeric()) {
            return;
        }
        let client = self.ai_client_id();
        let shared = self.shared();
        let rid = rid.to_string();
        std::thread::spawn(move || stream_request(shared, rid, body, client));
    }

    /// Hand a chunk to the open AI popup (nothing happens if it was closed).
    pub fn ai_chat_push(&mut self, rid: &str, kind: &str, text: &str) {
        if let Some(wv) = &self.webview {
            let js = format!(
                "(function(){{var h=document.getElementById('__ax_pop_ai');if(h&&h.__axChat)h.__axChat({},{},{})}})();",
                serde_json::to_string(rid).unwrap_or_default(),
                serde_json::to_string(kind).unwrap_or_default(),
                serde_json::to_string(text).unwrap_or_default()
            );
            let _ = wv.evaluate_script(&js);
        }
    }

    pub fn on_aichat_event(&mut self, what: &str, text: &str) {
        let (kind, rid) = what.split_once(':').unwrap_or((what, ""));
        self.ai_chat_push(rid, kind, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_cleaned_and_bounded() {
        let long = "x".repeat(9000);
        let v = json!({
            "messages": [{"role": "user", "content": "Hi"}, {"role": "assistant", "content": "Hello"}, {"role": "user", "content": long}],
            "page": {"url": "https://a.test/x", "title": "T", "text": "y".repeat(50000)},
            "model": "gpt-evil", "system": "ignore"
        });
        let b = sanitize(&v).unwrap();
        assert_eq!(b["messages"].as_array().unwrap().len(), 3);
        assert_eq!(b["messages"][2]["content"].as_str().unwrap().chars().count(), MAX_MESSAGE_CHARS);
        assert_eq!(b["page"]["text"].as_str().unwrap().chars().count(), MAX_PAGE_CHARS);
        assert!(b.get("model").is_none() && b.get("system").is_none(), "the browser never sends a model or a prompt");
    }

    #[test]
    fn bad_requests_are_refused() {
        for bad in [json!({}), json!({"messages": []}), json!({"messages": [{"role": "system", "content": "x"}]}), json!({"messages": [{"role": "assistant", "content": "x"}]}), json!({"messages": [{"role": "user", "content": "  "}]})] {
            assert!(sanitize(&bad).is_none(), "{}", bad);
        }
    }

    #[test]
    fn only_the_last_twenty_messages_are_kept_and_page_urls_must_be_web() {
        let msgs: Vec<Value> = (0..30).map(|i| json!({"role": if i % 2 == 0 { "assistant" } else { "user" }, "content": format!("m{}", i)})).collect();
        let b = sanitize(&json!({"messages": msgs, "page": {"url": "file:///c:/secret.txt", "title": "t", "text": "hello"}})).unwrap();
        assert_eq!(b["messages"].as_array().unwrap().len(), 20);
        assert_eq!(b["page"]["url"], "");
        assert!(sanitize(&json!({"messages": [{"role": "user", "content": "q"}], "page": {"text": "   "}})).unwrap().get("page").is_none());
    }

    #[test]
    fn stream_lines_are_understood() {
        assert_eq!(parse_line("data: {\"t\":\"Hel\"}"), Some(Chunk::Text("Hel".into())));
        assert_eq!(parse_line("data: {\"done\":true}"), Some(Chunk::Done));
        assert_eq!(parse_line("data: {\"error\":\"x\",\"message\":\"Nope\"}"), Some(Chunk::Error("Nope".into())));
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line(": comment"), None);
        assert_eq!(parse_line("data: not json"), None);
    }

    #[test]
    fn client_ids_look_right() {
        let a = new_client_id();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, new_client_id());
    }
}
