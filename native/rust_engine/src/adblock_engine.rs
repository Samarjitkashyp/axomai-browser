//! Built-in AdBlocker & Privacy Shield Engine for Axomai Browser.
//! Implements EasyList / EasyPrivacy network rule matching and cosmetic CSS element hiding.

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleAction {
    Block,
    Allow,
}

#[derive(Debug, Clone)]
pub struct NetworkFilter {
    pub pattern: String,
    pub action: RuleAction,
    pub is_domain_anchor: bool,
}

#[derive(Debug, Clone)]
pub struct AdBlockStats {
    pub blocked_requests_count: u64,
    pub total_inspected_requests: u64,
}

pub struct AdBlockEngine {
    pub is_enabled: bool,
    pub network_filters: Vec<NetworkFilter>,
    pub cosmetic_filters: HashSet<String>,
    pub stats: AdBlockStats,
}

impl AdBlockEngine {
    pub fn new() -> Self {
        let mut engine = AdBlockEngine {
            is_enabled: true,
            network_filters: Vec::new(),
            cosmetic_filters: HashSet::new(),
            stats: AdBlockStats {
                blocked_requests_count: 0,
                total_inspected_requests: 0,
            },
        };

        // Load standard default tracker & ad blocking patterns
        engine.add_default_rules();
        engine
    }

    fn add_default_rules(&mut self) {
        let default_patterns = [
            "doubleclick.net",
            "google-analytics.com",
            "googlesyndication.com",
            "adnxs.com",
            "facebook.com/tr/",
            "/pagead/",
            "/ads.js",
            "/advertisement/",
            "tracking.pixel",
        ];

        for pattern in default_patterns {
            self.network_filters.push(NetworkFilter {
                pattern: pattern.to_string(),
                action: RuleAction::Block,
                is_domain_anchor: false,
            });
        }

        // Default cosmetic element hiding selectors
        let default_cosmetics = [
            ".ad-banner",
            ".sponsored-post",
            "#ad-slot",
            ".google-ad",
            "[data-ad-unit]",
        ];
        for sel in default_cosmetics {
            self.cosmetic_filters.insert(sel.to_string());
        }
    }

    /// Check if a network request URL should be blocked
    pub fn should_block_url(&mut self, url: &str) -> bool {
        if !self.is_enabled {
            return false;
        }

        self.stats.total_inspected_requests += 1;

        for filter in &self.network_filters {
            if url.contains(&filter.pattern) {
                if filter.action == RuleAction::Block {
                    self.stats.blocked_requests_count += 1;
                    return true;
                } else {
                    return false; // Explicit allowlist rule
                }
            }
        }
        false
    }

    /// Generate combined CSS stylesheet for cosmetic ad hiding
    pub fn generate_cosmetic_css(&self) -> String {
        if !self.is_enabled || self.cosmetic_filters.is_empty() {
            return String::new();
        }

        let selectors: Vec<&str> = self.cosmetic_filters.iter().map(|s| s.as_str()).collect();
        format!("{} {{ display: none !important; visibility: hidden !important; }}", selectors.join(", "))
    }
}
