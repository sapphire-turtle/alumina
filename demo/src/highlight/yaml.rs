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
    let mut push = |r: Range<usize>, k: TokenKind| out.push((base + r.start..base + r.end, k));
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
        if is_blank(c) {
            i += 1;
        } else if c == b'#' && (i == 0 || is_blank(b[i - 1])) {
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
        } else if matches!(c, b'&' | b'*' | b'!') {
            let mut end = i + 1;
            while end < b.len()
                && !is_blank(b[end])
                && !(depth > 0 && matches!(b[end], b',' | b']' | b'}'))
            {
                end += 1;
            }
            push(i..end, TokenKind::Attribute);
            i = end;
        } else {
            let (end, kind) = scalar(line, i, depth);
            let after = end + if kind == TokenKind::Key { 1 } else { 0 };
            push(i..end, kind);
            if kind == TokenKind::Key {
                push(end..after, TokenKind::Punctuation);
            }
            i = after.max(i + 1);
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

/// The scalar starting at `from`: where it ends, and its kind (`Key` when a `:` follows,
/// which the caller marks).
fn scalar(line: &str, from: usize, depth: usize) -> (usize, TokenKind) {
    let b = line.as_bytes();
    let quote = b[from];
    let mut end = from;
    let quoted = quote == b'"' || quote == b'\'';
    if quoted {
        end += 1;
        while end < b.len() {
            if quote == b'"' && b[end] == b'\\' {
                end += 2;
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
    // A key is followed by its colon (a quoted key may have blanks before it).
    let mut colon = end;
    if quoted {
        while colon < b.len() && is_blank(b[colon]) {
            colon += 1;
        }
    }
    if colon < b.len() && key_colon(b, colon, depth) {
        // Trailing blanks of a plain key are not part of it.
        let key_end = if quoted {
            end
        } else {
            from + line[from..end].trim_end().len()
        };
        if colon == key_end {
            return (key_end, TokenKind::Key);
        }
    }
    let end = if quoted {
        end
    } else {
        from + line[from..end].trim_end().len()
    };
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
    (end, kind)
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
}
