//! A window for trying alumina's widgets by hand.
//!
//! [`Demo`] is the app; [`run`] opens it in a desktop window.

mod code;
mod datetime;
mod highlight;
mod markdown;

use corundum_framework_gui::egui;

/// Below this width the pages stack their parts, as a phone's would.
pub const NARROW: f32 = 600.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    DateTime,
    Markdown,
    Code,
}

/// The demo: a page per crate.
pub struct Demo {
    page: Page,
    datetime: datetime::DateTimeDemo,
    markdown: markdown::MarkdownDemo,
    code: code::CodeDemo,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            page: Page::DateTime,
            datetime: datetime::DateTimeDemo::default(),
            markdown: markdown::MarkdownDemo::default(),
            code: code::CodeDemo::default(),
        }
    }
}

impl corundum_framework_gui::eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut corundum_framework_gui::eframe::Frame) {
        self.pages(ui);
    }
}

impl Demo {
    fn pages(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("pages").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.page, Page::DateTime, "Date & time");
                ui.selectable_value(&mut self.page, Page::Markdown, "Markdown");
                ui.selectable_value(&mut self.page, Page::Code, "Code");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });
        egui::CentralPanel::default().show(ui, |ui| match self.page {
            Page::DateTime => self.datetime.ui(ui),
            Page::Markdown => self.markdown.ui(ui),
            Page::Code => self.code.ui(ui),
        });
    }
}

/// The demo's entry in the desktop's app menu, and the window's id and icon. A `cargo run`
/// puts no entry there; the icon it still gives the window.
const LAUNCHER: corundum_framework_platform::launcher::LauncherSpec =
    corundum_framework_platform::launcher::LauncherSpec {
        id: "alumina-demo",
        name: "Alumina Demo",
        description: "Try alumina's egui widgets",
        category: "Development",
        icon_svg: include_bytes!("../assets/app-icon.svg"),
        bundle_prefix: "org.sapphire-turtle",
    };

fn window_options() -> corundum_framework_gui::window::WindowOptions {
    corundum_framework_gui::window::WindowOptions::new("alumina demo")
        .launcher(LAUNCHER)
        .inner_size([960.0, 680.0])
        .min_inner_size([320.0, 400.0])
        // The Japanese labels need a CJK font, which egui does not bundle.
        .fonts(|ctx| {
            corundum_framework_gui::fonts::install_system_cjk_fallback(ctx);
        })
}

/// Log to standard error, `RUST_LOG` choosing what.
pub fn init_logging() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,alumina_demo=debug"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Open the demo's window and run it until it closes.
pub fn run() -> corundum_framework_gui::eframe::Result<()> {
    init_logging();
    tracing::info!("alumina demo: start");
    corundum_framework_gui::window::run(window_options(), |_cc| Box::new(Demo::default()))
}
