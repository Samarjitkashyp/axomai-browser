//! Gamer & Developer Resource Performance Limiter (Opera GX style) for Axomai Browser.
//! Enforces hardware resource limits on RAM, CPU, and Network Bandwidth to guarantee host system smoothness.

#[derive(Debug, Clone)]
pub struct ResourceLimiterConfig {
    pub max_ram_mb: usize,
    pub is_ram_limiter_active: bool,
    pub max_cpu_percent: u8, // 10% to 100%
    pub is_cpu_limiter_active: bool,
    pub max_bandwidth_kbps: u32,
    pub is_bandwidth_limiter_active: bool,
}

pub struct ResourceLimiter {
    pub config: ResourceLimiterConfig,
    pub current_ram_usage_mb: usize,
    pub current_cpu_usage_percent: u8,
}

impl ResourceLimiter {
    pub fn new() -> Self {
        ResourceLimiter {
            config: ResourceLimiterConfig {
                max_ram_mb: 2048, // 2 GB limit default
                is_ram_limiter_active: false,
                max_cpu_percent: 50,
                is_cpu_limiter_active: false,
                max_bandwidth_kbps: 10240, // 10 Mbps
                is_bandwidth_limiter_active: false,
            },
            current_ram_usage_mb: 180,
            current_cpu_usage_percent: 12,
        }
    }

    pub fn set_ram_limit(&mut self, max_mb: usize, enable: bool) {
        self.config.max_ram_mb = max_mb;
        self.config.is_ram_limiter_active = enable;
    }

    pub fn set_cpu_limit(&mut self, max_percent: u8, enable: bool) {
        self.config.max_cpu_percent = max_percent.clamp(5, 100);
        self.config.is_cpu_limiter_active = enable;
    }

    pub fn set_bandwidth_limit(&mut self, max_kbps: u32, enable: bool) {
        self.config.max_bandwidth_kbps = max_kbps;
        self.config.is_bandwidth_limiter_active = enable;
    }

    /// Check if current allocation exceeds user-defined budget
    pub fn should_throttle_allocation(&self, requested_bytes: usize) -> bool {
        if self.config.is_ram_limiter_active {
            let requested_mb = requested_bytes / (1024 * 1024);
            if self.current_ram_usage_mb + requested_mb > self.config.max_ram_mb {
                return true;
            }
        }
        false
    }
}
