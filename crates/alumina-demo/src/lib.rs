//! A window for trying alumina's widgets by hand, on a desktop or a phone.
//!
//! [`Demo`] is the app; [`run`] opens it in a desktop window (and on iOS, where
//! `alumina_demo_main` calls it), and `android_main` on an Android phone.

mod datetime;

use corundum_framework_gui::egui;

/// Below this width the pages stack their parts, as on a phone.
pub const NARROW: f32 = 600.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    DateTime,
    Markdown,
}

/// The demo: a page per crate.
pub struct Demo {
    page: Page,
    datetime: datetime::DateTimeDemo,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            page: Page::DateTime,
            datetime: datetime::DateTimeDemo::default(),
        }
    }
}

impl corundum_framework_gui::eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut corundum_framework_gui::eframe::Frame) {
        egui::Panel::top("pages").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.page, Page::DateTime, "Date & time");
                ui.selectable_value(&mut self.page, Page::Markdown, "Markdown");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });
        egui::CentralPanel::default().show(ui, |ui| match self.page {
            Page::DateTime => self.datetime.ui(ui),
            Page::Markdown => {
                ui.label("Not yet: see sapphire-turtle/alumina#1.");
            }
        });
    }
}

fn window_options() -> corundum_framework_gui::window::WindowOptions {
    corundum_framework_gui::window::WindowOptions::new("alumina demo")
        .inner_size([960.0, 680.0])
        .min_inner_size([320.0, 400.0])
        // The Japanese labels need a CJK font, which egui does not bundle.
        .fonts(|ctx| {
            corundum_framework_gui::fonts::install_system_cjk_fallback(ctx);
        })
}

/// Log to standard error, `RUST_LOG` choosing what.
#[cfg(not(target_os = "android"))]
pub fn init_logging() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,alumina_demo=debug"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Open the demo's window and run it until it closes.
#[cfg(not(target_os = "android"))]
pub fn run() -> corundum_framework_gui::eframe::Result<()> {
    init_logging();
    tracing::info!("alumina demo: start");
    corundum_framework_gui::window::run(window_options(), |_cc| Box::new(Demo::default()))
}

/// The iOS app's entry: `main.m` in ios/ hands the main thread to this.
#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn alumina_demo_main() {
    if let Err(err) = run() {
        tracing::error!("alumina demo: run failed: {err}");
    }
}

/// The Android app's entry, called by android-activity on GameActivity's thread.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: corundum_framework_gui::window::AndroidApp) {
    corundum_framework_gui::logcat::install("alumina-demo", "alumina_demo=debug");
    tracing::info!("alumina demo: start");
    if let Err(err) = corundum_framework_gui::window::run_android(app, window_options(), |_cc| {
        Box::new(Demo::default())
    }) {
        tracing::error!("alumina demo: run failed: {err}");
    }
}
