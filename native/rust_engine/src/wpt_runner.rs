//! W3C Web Platform Tests (WPT) Compliance Test Runner for Axomai Browser.
//! Executes official W3C specification tests and compiles automated compliance pass-rate reports.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WptStatus {
    Pass,
    Fail,
    Timeout,
    NotRun,
}

#[derive(Debug, Clone)]
pub struct WptAssertion {
    pub name: String,
    pub status: WptStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WptTestResult {
    pub test_path: String,
    pub status: WptStatus,
    pub subtests: Vec<WptAssertion>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct WptReport {
    pub suite_name: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub pass_rate_percentage: f32,
    pub results: Vec<WptTestResult>,
}

pub struct WptRunner {
    pub results: Vec<WptTestResult>,
}

impl WptRunner {
    pub fn new() -> Self {
        WptRunner {
            results: Vec::new(),
        }
    }

    pub fn record_test(&mut self, test_path: &str, status: WptStatus, subtests: Vec<WptAssertion>, duration_ms: u64) {
        self.results.push(WptTestResult {
            test_path: test_path.to_string(),
            status,
            subtests,
            duration_ms,
        });
    }

    pub fn generate_report(&self, suite_name: &str) -> WptReport {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.status == WptStatus::Pass).count();
        let failed = total - passed;
        let pass_rate = if total > 0 {
            (passed as f32 / total as f32) * 100.0
        } else {
            100.0
        };

        WptReport {
            suite_name: suite_name.to_string(),
            total_tests: total,
            passed_tests: passed,
            failed_tests: failed,
            pass_rate_percentage: pass_rate,
            results: self.results.clone(),
        }
    }
}
