//! A code editor for egui that keeps the indentation.
//!
//! The brackets `{ [ (` and `} ] )` are the only syntax it knows, so the same rules serve
//! JSON, Rust, JS, C and the like.
//!
//! [`CodeEditor`] is the egui widget. The edits behind it are functions on a source and a
//! byte selection with no egui in them:
//!
//! - [`Indent`]: the unit of indentation, a tab or some spaces, `Spaces(4)` by default.
//! - [`newline`]: Enter, which keeps the indentation, goes a level in after an opener and
//!   puts a closer between brackets on a line of its own.
//! - [`indent`], [`outdent`] and [`backspace`]: Tab, Shift+Tab and Backspace.
//! - [`close`]: a closing bracket typed on a blank indented line, which comes out a level.
//! - [`Edit`]: what they return.
//!
//! ```no_run
//! use alumina_code::{CodeEditor, Indent};
//!
//! struct Settings {
//!     indent: Indent,
//! }
//!
//! fn ui(ui: &mut egui::Ui, source: &mut String, settings: &Settings) {
//!     // The unit of indentation is the app's setting; the editor keeps none.
//!     let out = CodeEditor::new(source).indent(settings.indent).show(ui);
//!     if out.changed {
//!         // `source` was edited.
//!     }
//! }
//! ```

#![warn(missing_docs)]

mod editor;
mod indent;

pub use editor::{CodeEditor, CodeOutput};
pub use egui;
pub use indent::{Edit, Indent, backspace, close, indent, newline, outdent};
