//! Optional proxy server for all web pages (HTTP or SOCKS5). The web engine reads it when the browser starts, so a change
//! needs a restart. A proxy is not a VPN: it only moves the browser's own traffic through the server you give it.

use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxySpec {
    pub socks: bool,
    pub host: String,
    pub port: String,
}

impl ProxySpec {
    pub fn display(&self) -> String {
        format!("{}://{}:{}", if self.socks { "socks5" } else { "http" }, self.host, self.port)
    }
}

/// `host:port`, `http://host:port` or `socks5://host:port`. Anything else (paths, credentials, odd characters) is refused.
pub fn parse(input: &str) -> Option<ProxySpec> {
    let s = input.trim();
    if s.is_empty() || s.len() > 200 {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    let (socks, rest) = if let Some(r) = lower.strip_prefix("socks5://") {
        (true, r.to_string())
    } else if let Some(r) = lower.strip_prefix("http://") {
        (false, r.to_string())
    } else if lower.contains("://") {
        return None;
    } else {
        (false, lower)
    };
    let rest = rest.trim_end_matches('/');
    let (host, port) = rest.rsplit_once(':')?;
    let port_n: u32 = port.parse().ok()?;
    let host_ok = !host.is_empty() && host.len() <= 253 && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') && !host.starts_with('.') && !host.starts_with('-');
    (host_ok && (1..=65535).contains(&port_n)).then(|| ProxySpec { socks, host: host.to_string(), port: port_n.to_string() })
}

static STARTUP: OnceLock<Option<ProxySpec>> = OnceLock::new();

/// Remember the proxy the browser started with; every web view must be created with the same one.
pub fn init(setting: &str) {
    let _ = STARTUP.set(parse(setting));
}

pub fn startup() -> Option<ProxySpec> {
    STARTUP.get().cloned().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(socks: bool, host: &str, port: &str) -> Option<ProxySpec> {
        Some(ProxySpec { socks, host: host.into(), port: port.into() })
    }

    #[test]
    fn accepted_forms() {
        assert_eq!(parse("127.0.0.1:8080"), spec(false, "127.0.0.1", "8080"));
        assert_eq!(parse("  HTTP://Proxy.Example.com:3128/ "), spec(false, "proxy.example.com", "3128"));
        assert_eq!(parse("socks5://localhost:1080"), spec(true, "localhost", "1080"));
    }

    #[test]
    fn refused_forms() {
        for bad in ["", "host", "host:0", "host:70000", "host:abc", "ftp://h:1", "http://u:p@h:80", "ht tp:80", "http://h:80/path", ":80", "-h:80"] {
            assert_eq!(parse(bad), None, "{}", bad);
        }
    }

    #[test]
    fn display_round_trips() {
        let p = parse("socks5://a.b:9").unwrap();
        assert_eq!(parse(&p.display()), Some(p));
    }
}
