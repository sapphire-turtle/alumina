//! A code editor for egui that keeps the indentation.
//!
//! The brackets `{ [ (` and `} ] )` are the only syntax it knows, so the same rules serve
//! JSON, Rust, JS, C and the like.
//!
//! For now the crate holds the edits, as functions on a source and a byte selection with no
//! egui in them:
//!
//! - [`Indent`]: the unit of indentation, a tab or some spaces, `Spaces(4)` by default.
//! - [`newline`]: Enter, which keeps the indentation, goes a level in after an opener and
//!   puts a closer between brackets on a line of its own.
//! - [`indent`], [`outdent`] and [`backspace`]: Tab, Shift+Tab and Backspace.
//! - [`close`]: a closing bracket typed on a blank indented line, which comes out a level.
//! - [`Edit`]: what they return.

#![warn(missing_docs)]

mod indent;

pub use egui;
pub use indent::{Edit, Indent, backspace, close, indent, newline, outdent};
