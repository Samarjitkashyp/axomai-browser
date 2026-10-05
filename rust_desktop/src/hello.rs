//! Windows Hello (face, fingerprint or PIN) before passwords are copied or exported. It uses the Windows
//! `UserConsentVerifier`; when Windows Hello is not set up, the action is refused with an explanation rather than allowed.

use crate::app::App;
use crate::web::WebEvent;

#[derive(Debug, PartialEq, Eq)]
pub enum HelloResult {
    Verified,
    Denied,
    /// Windows Hello cannot be used here; the text is Windows' reason.
    Unavailable(String),
}

/// What the PowerShell helper prints: `result:<Verified|...>` or `unavailable:<reason>`.
pub fn parse_output(line: &str) -> HelloResult {
    let line = line.trim();
    if let Some(r) = line.strip_prefix("result:") {
        return if r.trim() == "Verified" { HelloResult::Verified } else { HelloResult::Denied };
    }
    if let Some(r) = line.strip_prefix("unavailable:") {
        return HelloResult::Unavailable(r.trim().to_string());
    }
    HelloResult::Unavailable("no answer".to_string())
}

pub fn explain(reason: &str) -> String {
    match reason {
        "DeviceNotPresent" | "NotConfiguredForUser" => "Windows Hello is not set up on this PC. Add a PIN or face / fingerprint sign-in in Windows Settings > Accounts > Sign-in options, or switch this protection off.".to_string(),
        "DisabledByPolicy" => "Windows Hello is switched off by your organisation's policy.".to_string(),
        other => format!("Windows Hello is not available ({}).", other),
    }
}

pub const SCRIPT: &str = "$ErrorActionPreference='Stop'; try { Add-Type -AssemblyName System.Runtime.WindowsRuntime; \
[void][Windows.Security.Credentials.UI.UserConsentVerifier,Windows.Security.Credentials.UI,ContentType=WindowsRuntime]; \
$as = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]; \
$a = $as.MakeGenericMethod([Windows.Security.Credentials.UI.UserConsentVerifierAvailability]).Invoke($null, @([Windows.Security.Credentials.UI.UserConsentVerifier]::CheckAvailabilityAsync())); $a.Wait(); \
if ($a.Result.ToString() -ne 'Available') { 'unavailable:' + $a.Result.ToString(); exit 0 }; \
$r = $as.MakeGenericMethod([Windows.Security.Credentials.UI.UserConsentVerificationResult]).Invoke($null, @([Windows.Security.Credentials.UI.UserConsentVerifier]::RequestVerificationAsync('Axomai Browser: confirm it is you to use your saved passwords'))); $r.Wait(); \
'result:' + $r.Result.ToString() } catch { 'unavailable:' + $_.Exception.Message }";

#[cfg(windows)]
fn spawn_check(shared: crate::web::WebShared, action: String, arg: String) {
    use std::os::windows::process::CommandExt;
    std::thread::spawn(move || {
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
            .creation_flags(0x0800_0000)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();
        let line = out.ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or("").to_string()).unwrap_or_default();
        shared.push_event(WebEvent::PageData("hello".into(), format!("{}\t{}", action, arg), line));
    });
}

#[cfg(not(windows))]
fn spawn_check(shared: crate::web::WebShared, action: String, arg: String) {
    shared.push_event(WebEvent::PageData("hello".into(), format!("{}\t{}", action, arg), "unavailable:not windows".into()));
}

impl App {
    fn toast_hello(&self, msg: &str) {
        if let Some(wv) = &self.webview {
            let shared = self.shared();
            self.core.toast(wv, msg, None, &shared);
        }
    }

    /// Returns true when the action may go ahead now. When protection is on, it starts the Windows prompt and returns
    /// false; the action runs again (through `hello_done`) once Windows says it was you.
    pub fn hello_guard(&mut self, action: &str, arg: &str) -> bool {
        if !self.settings.hello_passwords {
            return true;
        }
        if self.hello_ok_once {
            self.hello_ok_once = false;
            return true;
        }
        self.toast_hello("\u{1F510} Confirm with Windows Hello\u{2026}");
        spawn_check(self.shared(), action.to_string(), arg.to_string());
        false
    }

    pub fn hello_done(&mut self, what: &str, line: &str) {
        let (action, arg) = what.split_once('\t').unwrap_or((what, ""));
        match parse_output(line) {
            HelloResult::Verified => {
                self.hello_ok_once = true;
                match action {
                    "pw-export" => self.passwords_export(),
                    other => self.password_command(other, arg),
                }
                self.hello_ok_once = false;
            }
            HelloResult::Denied => self.toast_hello("Windows Hello did not confirm it is you, so nothing was done."),
            HelloResult::Unavailable(r) => self.toast_hello(&explain(&r)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_output_is_understood() {
        assert_eq!(parse_output("result:Verified\r\n"), HelloResult::Verified);
        assert_eq!(parse_output("result:Canceled"), HelloResult::Denied);
        assert_eq!(parse_output("result:RetriesExhausted"), HelloResult::Denied);
        assert_eq!(parse_output("unavailable:DeviceNotPresent"), HelloResult::Unavailable("DeviceNotPresent".into()));
        assert_eq!(parse_output(""), HelloResult::Unavailable("no answer".into()));
        assert_eq!(parse_output("something else"), HelloResult::Unavailable("no answer".into()));
    }

    #[test]
    fn reasons_are_explained() {
        assert!(explain("DeviceNotPresent").contains("Sign-in options"));
        assert!(explain("DisabledByPolicy").contains("policy"));
        assert!(explain("Weird").contains("Weird"));
    }

    #[test]
    fn script_asks_before_it_verifies() {
        let a = SCRIPT.find("CheckAvailabilityAsync").unwrap();
        let r = SCRIPT.find("RequestVerificationAsync").unwrap();
        assert!(a < r);
        assert!(SCRIPT.contains("'unavailable:'"));
    }
}
