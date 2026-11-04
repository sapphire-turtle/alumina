//! A Markdown editor for egui that formats as you type.
//!
//! The source is shown as it is, marks included (`#`, `**`, `- [ ]`), drawn in a faint
//! colour; only its look changes. It is egui's `TextEdit` with a layouter, so the cursor,
//! the selection, the IME and undo are egui's.
//!
//! - [`MarkdownEditor`]: the editor, or a read-only view with the same look (text can still
//!   be selected and copied). It reports a link clicked and leaves opening it to the app.
//! - [`MarkdownToolbar`]: a row of buttons for the same commands as the shortcuts, placed by
//!   the app and tied to an editor by its `egui::Id`. Labels are replaceable; the icons are
//!   the app's own ([`ToolbarIcons`]).
//! - [`command`], [`indent`] and [`newline`]: the edits behind both, as functions on a
//!   source and a selection, with no egui in them.
//! - [`parse`], [`MarkdownStyle`] and [`layout`]: the source read into blocks and marks, and
//!   turned into a `LayoutJob`.
//!
//! Headings, emphasis, strong, strikethrough, inline and fenced code, lists, task lists,
//! block quotes, links and thematic breaks are covered. Ticking a `[ ]` by clicking is not.
//!
//! ```no_run
//! use alumina_markdown::{MarkdownEditor, MarkdownToolbar, egui};
//!
//! fn note(ui: &mut egui::Ui, text: &mut String) {
//!     let id = egui::Id::unique("note");
//!     MarkdownToolbar::new(text, id).show(ui);
//!     let output = MarkdownEditor::new(text).id(id).show(ui);
//!     if let Some(url) = output.link_clicked {
//!         // Open `url`.
//!     }
//! }
//! ```

mod commands;
mod editor;
mod layout;
mod parse;
mod style;
mod toolbar;

pub use commands::{Command, Edit, command, indent, newline};
pub use editor::{MarkdownEditor, MarkdownOutput};
pub use egui;
pub use layout::layout;
pub use parse::{Block, BlockKind, Doc, Inline, InlineKind, parse};
pub use style::MarkdownStyle;
pub use toolbar::{MarkdownToolbar, ToolbarIcons, ToolbarLabels};
