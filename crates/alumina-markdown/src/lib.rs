//! A Markdown editor for egui that formats as you type (sapphire-turtle/alumina#1).
//!
//! So far: [`parse`], which reads a source into the blocks and marks the editor draws, and
//! [`layout`], which turns that into a `LayoutJob`.

mod layout;
mod parse;
mod style;

pub use egui;
pub use layout::layout;
pub use parse::{Block, BlockKind, Doc, Inline, InlineKind, parse};
pub use style::MarkdownStyle;
