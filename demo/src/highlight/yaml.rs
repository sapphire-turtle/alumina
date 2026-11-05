//! A line-based YAML highlighter: keys, scalars, comments, anchors, tags and flow
//! punctuation. It does not follow block scalars or multi-line flow collections.

use std::ops::Range;

use alumina_code::TokenKind;

/// The tokens of `code`, as byte ranges of it.
pub fn highlight(code: &str) -> Vec<(Range<usize>, TokenKind)> {
    let mut out = Vec::new();
    let mut start = 0;
    for raw in code.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let line = line.strip_suffix('\r').unwrap_or(line);
        highlight_line(line, start, &mut out);
        start += raw.len();
    }
    out
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

fn highlight_line(line: &str, base: usize, out: &mut Vec<(Range<usize>, TokenKind)>) {
    let b = line.as_bytes();
    // Never an empty token.
    let mut push = |r: Range<usize>, k: TokenKind| {
        if !r.is_empty() {
            out.push((base + r.start..base + r.end, k));
        }
    };
    let mut i = 0;
    while i < b.len() && is_blank(b[i]) {
        i += 1;
    }
    let trimmed = line[i..].trim_end();
    if trimmed == "---" || trimmed == "..." {
        push(i..i + 3, TokenKind::Punctuation);
        return;
    }
    // Block sequence dashes, possibly nested (`- - a`).
    while i < b.len() && b[i] == b'-' && (i + 1 == b.len() || is_blank(b[i + 1])) {
        push(i..i + 1, TokenKind::Punctuation);
        i += 1;
        while i < b.len() && is_blank(b[i]) {
            i += 1;
        }
    }
    let mut depth = 0usize;
    while i < b.len() {
        let c = b[i];
        // Any whitespace, such as U+3000 or U+00A0, steps over its whole char.
        let ch = line[i..].chars().next().unwrap_or(' ');
        if ch.is_whitespace() {
            i += ch.len_utf8();
        } else if c == b'#' && (i == 0 || line[..i].ends_with(char::is_whitespace)) {
            push(i..b.len(), TokenKind::Comment);
            return;
        } else if matches!(c, b'[' | b'{') {
            depth += 1;
            push(i..i + 1, TokenKind::Punctuation);
            i += 1;
        } else if matches!(c, b']' | b'}') {
            depth = depth.saturating_sub(1);
            push(i..i + 1, TokenKind::Punctuation);
            i += 1;
        } else if c == b',' && depth > 0 {
            push(i..i + 1, TokenKind::Punctuation);
            i += 1;
        } else if key_colon(b, i, depth) {
            // A colon with no key before it.
            push(i..i + 1, TokenKind::Punctuation);
            i += 1;
        } else if matches!(c, b'&' | b'*' | b'!') {
            let mut end = i + 1;
            while end < b.len() {
                let ch = line[end..].chars().next().unwrap_or(' ');
                if ch.is_whitespace() || (depth > 0 && matches!(ch, ',' | ']' | '}')) {
                    break;
                }
                end += ch.len_utf8();
            }
            push(i..end, TokenKind::Attribute);
            i = end;
        } else {
            let (end, kind, colon) = scalar(line, i, depth);
            push(i..end, kind);
            if let Some(colon) = colon {
                push(colon..colon + 1, TokenKind::Punctuation);
            }
            // `scalar` takes at least one char, so this always moves on.
            i = colon.map_or(end, |c| c + 1).max(i + 1);
            while !line.is_char_boundary(i) {
                i += 1;
            }
        }
    }
}

/// Whether the `:` at `at` ends a key: it is followed by a blank or the end (or, in a flow,
/// by a flow indicator).
fn key_colon(b: &[u8], at: usize, depth: usize) -> bool {
    b[at] == b':'
        && (at + 1 == b.len()
            || is_blank(b[at + 1])
            || (depth > 0 && matches!(b[at + 1], b',' | b']' | b'}')))
}

/// The scalar starting at `from` (not at a key's colon): where it ends, its kind, and for a
/// `Key` the position of its colon, which the caller marks.
fn scalar(line: &str, from: usize, depth: usize) -> (usize, TokenKind, Option<usize>) {
    let b = line.as_bytes();
    let quote = b[from];
    let mut end = from;
    let quoted = quote == b'"' || quote == b'\'';
    if quoted {
        end += 1;
        while end < b.len() {
            if quote == b'"' && b[end] == b'\\' {
                end += 1;
                // The escaped char, unless it is a multi-byte one (its bytes follow on).
                if b.get(end).is_some_and(u8::is_ascii) {
                    end += 1;
                }
                continue;
            }
            if b[end] == quote {
                if quote == b'\'' && b.get(end + 1) == Some(&b'\'') {
                    end += 2;
                    continue;
                }
                end += 1;
                break;
            }
            end += 1;
        }
        end = end.min(b.len());
    } else {
        while end < b.len() {
            let c = b[end];
            if key_colon(b, end, depth)
                || (c == b'#' && end > from && is_blank(b[end - 1]))
                || (depth > 0 && matches!(c, b',' | b']' | b'}'))
            {
                break;
            }
            end += 1;
        }
    }
    // Trailing whitespace of a plain scalar is not part of it.
    let text_end = if quoted {
        end
    } else {
        from + line[from..end].trim_end().len()
    };
    // A key is followed by its colon (a quoted key may have blanks before it).
    let mut colon = end;
    if quoted {
        while colon < b.len() && is_blank(b[colon]) {
            colon += 1;
        }
    }
    if text_end > from && colon < b.len() && key_colon(b, colon, depth) {
        return (text_end, TokenKind::Key, Some(colon));
    }
    let end = text_end;
    let text = &line[from..end];
    let kind = if quoted {
        TokenKind::String
    } else if matches!(text, "true" | "false" | "null" | "~") {
        TokenKind::Constant
    } else if is_number(text) {
        TokenKind::Number
    } else {
        TokenKind::String
    };
    (end, kind, None)
}

fn is_number(text: &str) -> bool {
    text.bytes().any(|c| c.is_ascii_digit())
        && text
            .bytes()
            .all(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.' | b'e' | b'E' | b'_'))
        && text.replace('_', "").parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(code: &str, text: &str, kind: TokenKind) -> bool {
        highlight(code)
            .iter()
            .any(|(r, k)| &code[r.clone()] == text && *k == kind)
    }

    const DOC: &str = "# c\nkey: value # note\nn: 12\nb: true\ns: \"q # not\"\n- item\na: &anc x\nr: *anc\nt: !tag y\n---\n";

    #[test]
    fn the_sample_is_marked() {
        use TokenKind::*;
        for (t, k) in [
            ("# c", Comment),
            ("key", Key),
            (":", Punctuation),
            ("value", String),
            ("# note", Comment),
            ("12", Number),
            ("true", Constant),
            ("\"q # not\"", String),
            ("-", Punctuation),
            ("&anc", Attribute),
            ("*anc", Attribute),
            ("!tag", Attribute),
            ("---", Punctuation),
        ] {
            assert!(has(DOC, t, k), "{t:?} should be {k:?}");
        }
    }

    #[test]
    fn a_quoted_hash_is_no_comment() {
        let toks = highlight(DOC);
        assert!(
            !toks
                .iter()
                .any(|(r, k)| *k == TokenKind::Comment && DOC[r.clone()].contains('q'))
        );
    }

    #[test]
    fn non_ascii_and_crlf() {
        assert!(has("名前: 太郎", "名前", TokenKind::Key));
        assert!(has("名前: 太郎", "太郎", TokenKind::String));
        let c = "a: 1\r\nb: x\r\n";
        assert!(has(c, "1", TokenKind::Number));
        assert!(has(c, "x", TokenKind::String));
        assert!(
            highlight(c)
                .iter()
                .all(|(r, _)| !c[r.clone()].contains('\r'))
        );
    }

    #[test]
    fn flow_and_words() {
        let c = "a: [1, null, ~]\nb: {x: no}\n";
        assert!(has(c, "[", TokenKind::Punctuation));
        assert!(has(c, ",", TokenKind::Punctuation));
        assert!(has(c, "null", TokenKind::Constant));
        assert!(has(c, "~", TokenKind::Constant));
        assert!(has(c, "no", TokenKind::String));
    }

    #[test]
    fn unicode_whitespace_and_stray_colons_are_safe() {
        for code in [
            "a: 1\u{3000}",
            "a: x\u{a0}",
            "名前: 太郎\u{3000}",
            ": x",
            "k : v",
            "k\u{3000}: v",
            "a: \"\\太\"",
            "a: \"\\太",
            "- \u{3000}# c",
        ] {
            for (r, _) in highlight(code) {
                assert!(!r.is_empty(), "{code:?}: empty token");
                assert!(
                    code.is_char_boundary(r.start) && code.is_char_boundary(r.end),
                    "{code:?}: {r:?} off a char boundary"
                );
            }
        }
        assert!(has("a: 1\u{3000}", "1", TokenKind::Number));
        assert!(has("k : v", "k", TokenKind::Key));
        assert!(has("a: x\u{a0}", "x", TokenKind::String));
    }
}
