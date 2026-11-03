//! A window for trying alumina's widgets by hand.

mod datetime;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("alumina demo")
            .with_inner_size([900.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "alumina-demo",
        options,
        Box::new(|_cc| Ok(Box::new(Demo::default()))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    DateTime,
    Markdown,
}

struct Demo {
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

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
