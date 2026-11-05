//! The demo's window.

#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> corundum_framework_gui::eframe::Result<()> {
    alumina_demo::run()
}
