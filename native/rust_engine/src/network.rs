use std::collections::HashMap;
use std::fs;
use std::io::Read;

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
        let raw = url_str.to_string();
        let mut scheme = "http".to_string();
        let mut rest = url_str;

        if url_str.starts_with("https://") {
            scheme = "https".to_string();
            rest = &url_str[8..];
        } else if url_str.starts_with("http://") {
            scheme = "http".to_string();
            rest = &url_str[7..];
        } else if url_str.starts_with("file://") {
            return Ok(URL {
                raw: raw.clone(),
                scheme: "file".to_string(),
                host: String::new(),
                port: 0,
                path: url_str[7..].to_string(),
                query: None,
                fragment: None,
            });
        } else if url_str.starts_with("data:") {
            return Ok(URL {
                raw: raw.clone(),
                scheme: "data".to_string(),
                host: String::new(),
                port: 0,
                path: url_str[5..].to_string(),
                query: None,
                fragment: None,
            });
        } else if url_str.starts_with("about:") {
            return Ok(URL {
                raw: raw.clone(),
                scheme: "about".to_string(),
                host: String::new(),
                port: 0,
                path: url_str[6..].to_string(),
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

        let (host_port, path) = if let Some(slash_idx) = without_query.find('/') {
            (&without_query[..slash_idx], without_query[slash_idx..].to_string())
        } else {
            (without_query, "/".to_string())
        };

        let (host, port) = if let Some(colon_idx) = host_port.find(':') {
            let h = host_port[..colon_idx].to_string();
            let p: u16 = host_port[colon_idx + 1..]
                .parse()
                .map_err(|e| format!("Invalid port: {}", e))?;
            (h, p)
        } else {
            let p = if scheme == "https" { 443 } else { 80 };
            (host_port.to_string(), p)
        };

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
        if relative_url.is_empty() {
            return format!("{}://{}{}", self.scheme, self.host, self.path);
        }
        if relative_url.contains("://")
            || relative_url.starts_with("data:")
            || relative_url.starts_with("file:")
        {
            return relative_url.to_string();
        }
        if relative_url.starts_with("//") {
            return format!("{}:{}", self.scheme, relative_url);
        }

        let port_suffix = if self.port != 80 && self.port != 443 && self.port != 0 {
            format!(":{}", self.port)
        } else {
            String::new()
        };

        if relative_url.starts_with('/') {
            return format!("{}://{}{}{}", self.scheme, self.host, port_suffix, relative_url);
        }

        let dir_path = if let Some(last_slash) = self.path.rfind('/') {
            &self.path[..last_slash]
        } else {
            ""
        };

        let formatted_dir = if dir_path.starts_with('/') {
            dir_path.to_string()
        } else {
            format!("/{}", dir_path)
        };

        let full_path = format!("{}/{}", formatted_dir, relative_url).replace("//", "/");
        format!("{}://{}{}{}", self.scheme, self.host, port_suffix, full_path)
    }

    pub fn request(&self) -> (HashMap<String, String>, String) {
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

        let target_url = format!("{}://{}{}{}", self.scheme, self.host, 
            if self.port != 80 && self.port != 443 { format!(":{}", self.port) } else { String::new() }, 
            self.path);

        match ureq::get(&target_url)
            .set("User-Agent", "AxomaiBrowser/1.0 (Rust Engine)")
            .call()
        {
            Ok(response) => {
                let mut headers = HashMap::new();
                for key in response.headers_names() {
                    if let Some(val) = response.header(&key) {
                        headers.insert(key.to_lowercase(), val.to_string());
                    }
                }
                match response.into_string() {
                    Ok(body) => (headers, body),
                    Err(e) => (headers, format!("Error reading response body: {}", e)),
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
                    // Try decoding base64 if standard
                    let cleaned = body_str.replace('\n', "").replace('\r', "");
                    // Simple base64 fallback or raw bytes
                    return (HashMap::new(), cleaned.into_bytes());
                }
                return (HashMap::new(), body_str.as_bytes().to_vec());
            }
            return (HashMap::new(), self.path.as_bytes().to_vec());
        }

        let target_url = format!("{}://{}{}{}", self.scheme, self.host, 
            if self.port != 80 && self.port != 443 { format!(":{}", self.port) } else { String::new() }, 
            self.path);

        match ureq::get(&target_url)
            .set("User-Agent", "AxomaiBrowser/1.0 (Rust Engine)")
            .call()
        {
            Ok(response) => {
                let mut headers = HashMap::new();
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
