//! The demo's highlighter: YAML by hand, Rust over syntect. alumina-code knows no language;
//! this is the app's side of [`alumina_code::Highlighter`].

use std::ops::Range;

use alumina_code::{Highlighter, TokenKind};

mod rust;
mod yaml;

/// Highlights the languages the demo shows.
#[derive(Default)]
pub struct DemoHighlighter;

impl Highlighter for DemoHighlighter {
    fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
        match language.to_ascii_lowercase().as_str() {
            "yaml" | "yml" => Some(yaml::highlight(code)),
            "rust" | "rs" => Some(rust::highlight(code)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_are_matched_without_case() {
        let h = DemoHighlighter;
        assert!(h.highlight("YML", "a: 1").is_some());
        assert!(h.highlight("Rust", "fn a() {}").is_some());
        assert!(h.highlight("json", "{}").is_none());
    }
}
