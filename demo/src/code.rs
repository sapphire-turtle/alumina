//! The page for alumina-code: the editor on a JSON, a YAML and a Rust sample.

use crate::highlight::DemoHighlighter;
use alumina_code::{CodeEditor, Indent};
use corundum_framework_gui::egui;

const JSON: &str = include_str!("code-sample.json");
const YAML: &str = include_str!("code-sample.yaml");
const RUST: &str = include_str!("code-sample.rs.txt");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sample {
    Json,
    Yaml,
    Rust,
}

impl Sample {
    /// The unit the sample is written in.
    fn indent(self) -> Indent {
        match self {
            Sample::Json | Sample::Yaml => Indent::Spaces(2),
            Sample::Rust => Indent::Spaces(4),
        }
    }

    /// The language the highlighter is asked for.
    fn language(self) -> &'static str {
        match self {
            Sample::Json => "json",
            Sample::Yaml => "yaml",
            Sample::Rust => "rust",
        }
    }
}

pub struct CodeDemo {
    sample: Sample,
    json: String,
    json_id: egui::Id,
    yaml: String,
    yaml_id: egui::Id,
    rust: String,
    rust_id: egui::Id,
    indent: Indent,
    read_only: bool,
    highlighter: DemoHighlighter,
}

impl Default for CodeDemo {
    fn default() -> Self {
        Self {
            sample: Sample::Json,
            json: JSON.to_owned(),
            json_id: egui::Id::unique("demo-code-json"),
            yaml: YAML.to_owned(),
            yaml_id: egui::Id::unique("demo-code-yaml"),
            rust: RUST.to_owned(),
            rust_id: egui::Id::unique("demo-code-rust"),
            indent: Sample::Json.indent(),
            read_only: false,
            highlighter: DemoHighlighter,
        }
    }
}

impl CodeDemo {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let before = self.sample;
            ui.selectable_value(&mut self.sample, Sample::Json, "JSON");
            ui.selectable_value(&mut self.sample, Sample::Yaml, "YAML");
            ui.selectable_value(&mut self.sample, Sample::Rust, "Rust");
            if self.sample != before {
                // Each sample starts in its own unit; the choice below is free after that.
                self.indent = self.sample.indent();
            }
            ui.separator();
            ui.selectable_value(&mut self.indent, Indent::Spaces(2), "2 spaces");
            ui.selectable_value(&mut self.indent, Indent::Spaces(4), "4 spaces");
            ui.selectable_value(&mut self.indent, Indent::Tab, "Tab");
            ui.separator();
            ui.checkbox(&mut self.read_only, "Read-only");
        });
        ui.separator();
        let (text, id) = match self.sample {
            Sample::Json => (&mut self.json, self.json_id),
            Sample::Yaml => (&mut self.yaml, self.yaml_id),
            Sample::Rust => (&mut self.rust, self.rust_id),
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            CodeEditor::new(text)
                .id(id)
                .language(self.sample.language())
                .highlighter(&self.highlighter)
                .read_only(self.read_only)
                .indent(self.indent)
                .desired_rows(20)
                .accessible_label("Code")
                .show(ui);
        });
    }
}
