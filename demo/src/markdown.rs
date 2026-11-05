//! The page for alumina-markdown: the formatted editor on a sample.

use crate::highlight::DemoHighlighter;
use alumina_markdown::{MarkdownEditor, MarkdownToolbar};
use corundum_framework_gui::egui;

const SAMPLE: &str = include_str!("markdown-sample.md");

pub struct MarkdownDemo {
    text: String,
    id: egui::Id,
    read_only: bool,
    last_link: Option<String>,
    highlighter: DemoHighlighter,
}

impl Default for MarkdownDemo {
    fn default() -> Self {
        Self {
            text: SAMPLE.to_owned(),
            id: egui::Id::unique("demo-markdown"),
            read_only: false,
            last_link: None,
            highlighter: DemoHighlighter,
        }
    }
}

impl MarkdownDemo {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.read_only, "Read-only");
        if let Some(link) = &self.last_link {
            ui.label(format!("last link clicked: {link}"));
        }
        if !self.read_only {
            MarkdownToolbar::new(&mut self.text, self.id).show(ui);
        }
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let out = MarkdownEditor::new(&mut self.text)
                .id(self.id)
                .highlighter(&self.highlighter)
                .read_only(self.read_only)
                .desired_rows(20)
                .show(ui);
            if let Some(link) = out.link_clicked {
                self.last_link = Some(link);
            }
        });
    }
}
