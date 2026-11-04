//! A Markdown editor for egui that formats as you type (sapphire-turtle/alumina#1).
//!
//! So far: [`parse`], which reads a source into the blocks and marks the editor draws.

mod parse;

pub use egui;
pub use parse::{Block, BlockKind, Doc, Inline, InlineKind, parse};
