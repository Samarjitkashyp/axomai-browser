// Puts the Axomai icon and version information into axomai_browser.exe, so Explorer, the Start menu shortcut, the
// taskbar and "Installed apps" show the Axomai icon instead of the generic one.
fn main() {
    println!("cargo:rerun-if-changed=assets/axomai.rc");
    println!("cargo:rerun-if-changed=assets/axomai.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").map_or(false, |os| os == "windows") {
        embed_resource::compile("assets/axomai.rc", embed_resource::NONE);
    }
}
