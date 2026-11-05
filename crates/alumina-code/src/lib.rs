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
//! # Highlighting
//!
//! The editor knows no language. The app implements [`Highlighter`], which turns a language
//! name and some code into [`TokenKind`] ranges, and the editor colours them by a
//! [`CodeTheme`] (which follows the visuals unless the app sets one). The highlighter runs
//! when the text, the language, the theme, the width or the font changes, not every frame;
//! a swapped highlighter takes effect at the next such change.
//!
//! ```no_run
//! use std::ops::Range;
//! use alumina_code::{CodeEditor, Highlighter, TokenKind};
//!
//! struct Fns;
//! impl Highlighter for Fns {
//!     fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
//!         (language == "rust").then(|| {
//!             code.match_indices("fn").map(|(i, m)| (i..i + m.len(), TokenKind::Keyword)).collect()
//!         })
//!     }
//! }
//!
//! fn ui(ui: &mut egui::Ui, source: &mut String) {
//!     CodeEditor::new(source).language("rust").highlighter(&Fns).show(ui);
//! }
//! ```
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
mod highlight;
mod indent;

pub use editor::{CodeEditor, CodeOutput};
pub use egui;
pub use highlight::{CodeTheme, Highlighter, TokenKind, highlighted_job};
pub use indent::{Edit, Indent, backspace, close, indent, newline, outdent};
