use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct URL {
    pub raw: String,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl URL {
    pub fn parse(url_str: &str) -> Result<Self, String> {
        let trimmed = url_str.trim();
        let raw = trimmed.to_string();
        let mut scheme = "http".to_string();
        let mut rest = trimmed;

        if trimmed.to_lowercase().starts_with("https://") {
            scheme = "https".to_string();
            rest = &trimmed[8..];
        } else if trimmed.to_lowercase().starts_with("http://") {
            scheme = "http".to_string();
            rest = &trimmed[7..];
        } else if trimmed.to_lowercase().starts_with("file://") {
            return Ok(URL {
                raw: raw.clone(),
                scheme: "file".to_string(),
                host: String::new(),
                port: 0,
                path: trimmed[7..].to_string(),
                query: None,
                fragment: None,
            });
        } else if trimmed.to_lowercase().starts_with("data:") {
            return Ok(URL {
                raw: raw.clone(),
                scheme: "data".to_string(),
                host: String::new(),
                port: 0,
                path: trimmed[5..].to_string(),
                query: None,
                fragment: None,
            });
        } else if trimmed.to_lowercase().starts_with("about:") {
            return Ok(URL {
                raw: raw.clone(),
                scheme: "about".to_string(),
                host: String::new(),
                port: 0,
                path: trimmed[6..].to_string(),
                query: None,
                fragment: None,
            });
        }

        // Extract fragment '#'
        let (without_fragment, fragment) = if let Some(hash_idx) = rest.find('#') {
            (&rest[..hash_idx], Some(rest[hash_idx + 1..].to_string()))
        } else {
            (rest, None)
        };

        // Extract query '?'
        let (without_query, query) = if let Some(q_idx) = without_fragment.find('?') {
            (&without_fragment[..q_idx], Some(without_fragment[q_idx + 1..].to_string()))
        } else {
            (without_fragment, None)
        };

        let (host_port, raw_path) = if let Some(slash_idx) = without_query.find('/') {
            (&without_query[..slash_idx], without_query[slash_idx..].to_string())
        } else {
            (without_query, "/".to_string())
        };

        let (host, port) = if let Some(colon_idx) = host_port.find(':') {
            let h = host_port[..colon_idx].to_lowercase();
            let p: u16 = host_port[colon_idx + 1..]
                .parse()
                .map_err(|e| format!("Invalid port: {}", e))?;
            (h, p)
        } else {
            let p = if scheme == "https" { 443 } else { 80 };
            (host_port.to_lowercase(), p)
        };

        let path = normalize_path(&raw_path);

        Ok(URL {
            raw,
            scheme,
            host,
            port,
            path,
            query,
            fragment,
        })
    }

    pub fn as_string(&self) -> String {
        if !self.raw.is_empty() {
            return self.raw.clone();
        }
        let port_suffix = if (self.scheme == "http" && self.port == 80)
            || (self.scheme == "https" && self.port == 443)
            || self.port == 0
        {
            String::new()
        } else {
            format!(":{}", self.port)
        };
        let q = self.query.as_ref().map(|q| format!("?{}", q)).unwrap_or_default();
        let f = self.fragment.as_ref().map(|f| format!("#{}", f)).unwrap_or_default();
        format!("{}://{}{}{}{}{}", self.scheme, self.host, port_suffix, self.path, q, f)
    }

    pub fn origin(&self) -> String {
        if self.scheme == "file" || self.scheme == "data" || self.scheme == "about" {
            return "null".to_string();
        }
        let port_suffix = if (self.scheme == "http" && self.port == 80)
            || (self.scheme == "https" && self.port == 443)
            || self.port == 0
        {
            String::new()
        } else {
            format!(":{}", self.port)
        };
        format!("{}://{}{}", self.scheme, self.host, port_suffix)
    }

    pub fn resolve(&self, relative_url: &str) -> String {
        let trimmed = relative_url.trim();
        if trimmed.is_empty() {
            return self.as_string();
        }
        if trimmed.contains("://")
            || trimmed.starts_with("data:")
            || trimmed.starts_with("file:")
            || trimmed.starts_with("about:")
        {
            return trimmed.to_string();
        }
        if trimmed.starts_with("//") {
            return format!("{}:{}", self.scheme, trimmed);
        }

        let port_suffix = if self.port != 80 && self.port != 443 && self.port != 0 {
            format!(":{}", self.port)
        } else {
            String::new()
        };

        if trimmed.starts_with('/') {
            let normalized = normalize_path(trimmed);
            return format!("{}://{}{}{}", self.scheme, self.host, port_suffix, normalized);
        }

        let dir_path = if let Some(last_slash) = self.path.rfind('/') {
            &self.path[..last_slash]
        } else {
            ""
        };

        let combined = format!("{}/{}", dir_path, trimmed);
        let normalized = normalize_path(&combined);
        format!("{}://{}{}{}", self.scheme, self.host, port_suffix, normalized)
    }

    pub fn request_url(&self) -> String {
        let port_suffix = if (self.scheme == "http" && self.port == 80)
            || (self.scheme == "https" && self.port == 443)
            || self.port == 0
        {
            String::new()
        } else {
            format!(":{}", self.port)
        };
        let q = self
            .query
            .as_ref()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();
        format!("{}://{}{}{}{}", self.scheme, self.host, port_suffix, self.path, q)
    }

    pub fn request(&self) -> (HashMap<String, String>, String) {
        if self.scheme == "about" {
            return (
                HashMap::new(),
                "<html><head><title>About</title></head><body></body></html>".to_string(),
            );
        }

        if self.scheme == "file" {
            match fs::read_to_string(&self.path) {
                Ok(content) => return (HashMap::new(), content),
                Err(e) => {
                    return (
                        HashMap::new(),
                        format!(
                            "<html><body><h1>Error loading file</h1><p>{}</p></body></html>",
                            e
                        ),
                    )
                }
            }
        }

        if self.scheme == "data" {
            if let Some(comma_idx) = self.path.find(',') {
                let header = self.path[..comma_idx].to_string();
                let body = self.path[comma_idx + 1..].to_string();
                let mut headers = HashMap::new();
                headers.insert("content-type".to_string(), header);
                return (headers, body);
            }
            return (HashMap::new(), self.path.clone());
        }

        let target_url = self.request_url();
        let agent = ureq::AgentBuilder::new()
            .redirects(5)
            .timeout(Duration::from_secs(10))
            .build();

        match agent
            .get(&target_url)
            .set("User-Agent", "AxomaiBrowser/1.0 (Rust Engine)")
            .call()
        {
            Ok(response) => {
                let mut headers = HashMap::new();
                headers.insert("status".to_string(), response.status().to_string());
                for key in response.headers_names() {
                    if let Some(val) = response.header(&key) {
                        headers.insert(key.to_lowercase(), val.to_string());
                    }
                }
                match response.into_string() {
                    Ok(body) => (headers, body),
                    Err(e) => (headers, format!("<html><body><h1>Error reading response body</h1><p>{}</p></body></html>", e)),
                }
            }
            Err(e) => (
                HashMap::new(),
                format!(
                    "<html><body><h1>Network Error</h1><p>Failed to load {}: {}</p></body></html>",
                    target_url, e
                ),
            ),
        }
    }

    pub fn request_bytes(&self) -> (HashMap<String, String>, Vec<u8>) {
        if self.scheme == "file" {
            match fs::read(&self.path) {
                Ok(bytes) => return (HashMap::new(), bytes),
                Err(_) => return (HashMap::new(), Vec::new()),
            }
        }

        if self.scheme == "data" {
            if let Some(comma_idx) = self.path.find(',') {
                let header = self.path[..comma_idx].to_string();
                let body_str = &self.path[comma_idx + 1..];
                if header.contains(";base64") {
                    let cleaned = body_str.replace('\n', "").replace('\r', "");
                    if let Some(bytes) = base64_decode(&cleaned) {
                        return (HashMap::new(), bytes);
                    }
                    return (HashMap::new(), cleaned.into_bytes());
                }
                return (HashMap::new(), body_str.as_bytes().to_vec());
            }
            return (HashMap::new(), self.path.as_bytes().to_vec());
        }

        let target_url = self.request_url();
        let agent = ureq::AgentBuilder::new()
            .redirects(5)
            .timeout(Duration::from_secs(10))
            .build();

        match agent
            .get(&target_url)
            .set("User-Agent", "AxomaiBrowser/1.0 (Rust Engine)")
            .call()
        {
            Ok(response) => {
                let mut headers = HashMap::new();
                headers.insert("status".to_string(), response.status().to_string());
                for key in response.headers_names() {
                    if let Some(val) = response.header(&key) {
                        headers.insert(key.to_lowercase(), val.to_string());
                    }
                }
                let mut bytes = Vec::new();
                let _ = response.into_reader().read_to_end(&mut bytes);
                (headers, bytes)
            }
            Err(_) => (HashMap::new(), Vec::new()),
        }
    }
}

fn normalize_path(path: &str) -> String {
    let mut segments = Vec::new();
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        } else if seg == ".." {
            segments.pop();
        } else {
            segments.push(seg);
        }
    }
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

pub fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        result.push(TABLE[(b0 >> 2) as usize] as char);
        result.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            result.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(TABLE[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut table = [255u8; 256];
    for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".iter().enumerate() {
        table[c as usize] = i as u8;
    }

    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 {
        return None;
    }

    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let c0 = table[chunk[0] as usize];
        let c1 = table[chunk[1] as usize];
        let c2 = if chunk[2] == b'=' { 0 } else { table[chunk[2] as usize] };
        let c3 = if chunk[3] == b'=' { 0 } else { table[chunk[3] as usize] };

        if c0 == 255 || c1 == 255 || (chunk[2] != b'=' && c2 == 255) || (chunk[3] != b'=' && c3 == 255) {
            return None;
        }

        let triple = ((c0 as u32) << 18) | ((c1 as u32) << 12) | ((c2 as u32) << 6) | (c3 as u32);
        out.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            out.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            out.push((triple & 0xFF) as u8);
        }
    }
    Some(out)
}
