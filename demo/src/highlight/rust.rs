//! Rust highlighting over syntect: its syntax parses the code, and the scopes it opens
//! become [`TokenKind`]s.

use std::ops::Range;
use std::sync::LazyLock;

use alumina_code::TokenKind;
use syntect::easy::ScopeRangeIterator;
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxSet};
use syntect::util::LinesWithEndings;

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// syntect scopes the primitive types `storage.type.rust`, as it does `let`; the text tells
/// them apart.
const PRIMITIVES: [&str; 17] = [
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char", "str",
];

/// The kind of a scope, by the first rule whose prefix it starts with.
fn kind_of(scope: Scope, text: &str) -> Option<TokenKind> {
    let name = scope.build_string();
    let is = |prefix: &str| name.starts_with(prefix);
    Some(if is("comment") {
        TokenKind::Comment
    } else if is("string") {
        TokenKind::String
    } else if is("constant.numeric") {
        TokenKind::Number
    } else if is("constant.language") || is("constant.character") {
        TokenKind::Constant
    } else if is("entity.name.function")
        || is("support.function")
        || is("support.macro")
        || is("entity.name.macro")
    {
        TokenKind::Function
    } else if is("entity.name.type")
        || is("entity.name.struct")
        || is("entity.name.enum")
        || is("entity.name.union")
        || is("entity.name.trait")
        || is("support.type")
        || (name == "storage.type.rust" && PRIMITIVES.contains(&text))
    {
        TokenKind::Type
    } else if is("keyword") || is("storage") {
        TokenKind::Keyword
    } else if is("meta.attribute") || is("meta.annotation") {
        TokenKind::Attribute
    } else if is("punctuation") && !is("punctuation.definition") {
        // The delimiters of a string or comment (`punctuation.definition.*`) stay with it.
        TokenKind::Punctuation
    } else {
        return None;
    })
}

/// The tokens of `code`, as byte ranges of it; each run of text gets the kind of its
/// innermost scope that maps to one.
pub fn highlight(code: &str) -> Vec<(Range<usize>, TokenKind)> {
    let Some(syntax) = SYNTAXES.find_syntax_by_extension("rs") else {
        return Vec::new();
    };
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut out: Vec<(Range<usize>, TokenKind)> = Vec::new();
    let mut line_start = 0;
    let mut push = |range: Range<usize>, kind: TokenKind| match out.last_mut() {
        // Delimiters and their contents are one run: `"x"`, not `"`, `x`, `"`.
        Some((last, k)) if *k == kind && last.end == range.start => last.end = range.end,
        _ => out.push((range, kind)),
    };
    'lines: for line in LinesWithEndings::from(code) {
        let Ok(ops) = state.parse_line(line, &SYNTAXES) else {
            // The rest cannot be parsed: leave it plain.
            break 'lines;
        };
        let text_len = line.trim_end_matches(['\r', '\n']).len();
        for (range, op) in ScopeRangeIterator::new(&ops, line) {
            if stack.apply(op).is_err() {
                break 'lines;
            }
            // The line ending is no part of a token.
            let range = range.start..range.end.min(text_len);
            if range.is_empty() {
                continue;
            }
            if let Some(kind) = stack
                .as_slice()
                .iter()
                .rev()
                .find_map(|s| kind_of(*s, &line[range.clone()]))
            {
                push(line_start + range.start..line_start + range.end, kind);
            }
        }
        line_start += line.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_is_marked() {
        let code = "fn main() {\n    let s = \"x\"; // c\n    println!(\"{}\", 1);\n}\n";
        let toks = highlight(code);
        let covers = |text: &str, kind: TokenKind| {
            let at = code.find(text).unwrap();
            toks.iter()
                .any(|(r, k)| *k == kind && r.start <= at && at + text.len() <= r.end)
        };
        assert!(covers("fn", TokenKind::Keyword));
        assert!(covers("let", TokenKind::Keyword));
        assert!(covers("main", TokenKind::Function));
        assert!(covers("\"x\"", TokenKind::String));
        assert!(covers("// c", TokenKind::Comment));
        assert!(covers("1)", TokenKind::Number) || covers("1", TokenKind::Number));
    }

    #[test]
    fn types_are_marked() {
        let code = "let x: i32 = 1; struct A { b: Vec<String> }\n";
        let toks = highlight(code);
        let covers = |text: &str, kind: TokenKind| {
            let at = code.find(text).unwrap();
            toks.iter()
                .any(|(r, k)| *k == kind && r.start <= at && at + text.len() <= r.end)
        };
        assert!(covers("let", TokenKind::Keyword));
        assert!(covers("struct", TokenKind::Keyword));
        assert!(covers("i32", TokenKind::Type));
        assert!(covers("A", TokenKind::Type));
    }
}
