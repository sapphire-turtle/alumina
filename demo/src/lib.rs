//! A window for trying alumina's widgets by hand.
//!
//! [`Demo`] is the app; [`run`] opens it in a desktop window.

mod code;
mod datetime;
mod highlight;
mod markdown;

use eframe::egui;

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

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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

/// The window: its title, size, and id and icon (the id lets a Wayland desktop match the
/// window to its icon). Japanese labels need no font set up here: eframe's `system_fonts`
/// finds a system font for glyphs egui's own fonts lack.
fn native_options() -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_title("alumina demo")
        .with_app_id("alumina-demo")
        .with_inner_size([960.0, 680.0])
        .with_min_inner_size([320.0, 400.0])
        .with_icon(icon());
    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

/// The window's icon, rendered from assets/app-icon.svg by build.rs.
fn icon() -> egui::IconData {
    let size: u32 = env!("ICON_SIZE").parse().expect("build.rs sets ICON_SIZE");
    egui::IconData {
        rgba: include_bytes!(concat!(env!("OUT_DIR"), "/app-icon.rgba")).to_vec(),
        width: size,
        height: size,
    }
}

/// Log to standard error, `RUST_LOG` choosing what.
pub fn init_logging() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,alumina_demo=debug"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Open the demo's window and run it until it closes.
pub fn run() -> eframe::Result<()> {
    init_logging();
    tracing::info!("alumina demo: start");
    eframe::run_native(
        "alumina-demo",
        native_options(),
        Box::new(|_cc| Ok(Box::new(Demo::default()))),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_icon_is_a_full_square_with_the_mark_drawn() {
        let icon = super::icon();
        assert_eq!(icon.width, icon.height);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        assert!(icon.rgba.chunks(4).any(|p| p[3] == 255), "nothing is drawn");
        assert!(icon.rgba.chunks(4).any(|p| p[3] == 0), "no transparent corner");
    }
}
