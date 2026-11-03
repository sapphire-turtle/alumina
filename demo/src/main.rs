//! The demo on a desktop.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn main() -> corundum_framework_gui::eframe::Result<()> {
    alumina_demo::run()
}

// On Android the demo starts from GameActivity (`android_main` in the lib), and on iOS from
// the Xcode app in ios/ (`alumina_demo_main` in the lib), not from here.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {}
