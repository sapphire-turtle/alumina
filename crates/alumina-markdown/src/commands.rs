//! Commands on a source and a byte selection: each returns an [`Edit`] to apply.
//!
//! Everything here works in bytes on char boundaries; the editor converts egui's char
//! cursors at its edge.

use std::ops::Range;

/// One change to a source: replace a range, then select a range of the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// The bytes of the source to replace.
    pub replace: Range<usize>,
    /// What goes in their place.
    pub with: String,
    /// The selection afterwards, in the edited text.
    pub selection: Range<usize>,
}

impl Edit {
    /// Apply the replacement to `text`.
    pub fn apply(&self, text: &mut String) {
        text.replace_range(self.replace.clone(), &self.with);
    }
}

/// Something the editor can do to the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// `**bold**`.
    Strong,
    /// `*italic*`.
    Emphasis,
    /// `~~struck~~`.
    Strikethrough,
    /// `` `code` ``.
    Code,
    /// `[text](url)`, with `url` selected.
    Link,
    /// `#` marks, 1 to 6; 0 removes the heading.
    Heading(u8),
    /// `- ` lines.
    BulletList,
    /// `1. ` lines.
    NumberedList,
    /// `- [ ] ` lines.
    TaskList,
    /// `> ` lines.
    Quote,
    /// A fenced code block.
    CodeBlock,
}

/// The edit that `command` makes to `selection` of `source`, or `None` if there is nothing
/// to do. A selection out of range, reversed or off a char boundary is clamped.
pub fn command(source: &str, selection: Range<usize>, command: Command) -> Option<Edit> {
    let sel = clamp(source, selection);
    match command {
        Command::Strong => Some(toggle(source, sel, "**")),
        Command::Emphasis => Some(toggle(source, sel, "*")),
        Command::Strikethrough => Some(toggle(source, sel, "~~")),
        Command::Code => Some(toggle(source, sel, "`")),
        Command::Link => Some(unlink(source, &sel).unwrap_or_else(|| link(source, sel))),
        line_command @ (Command::Heading(_)
        | Command::BulletList
        | Command::NumberedList
        | Command::TaskList
        | Command::Quote
        | Command::CodeBlock) => lines::edit(source, sel, line_command),
    }
}

/// Enter at the byte `cursor`: the edit that continues (or, on an empty item, leaves) the
/// list item it is in. `None` if the cursor is not in a list item, so the editor inserts
/// the newline itself.
pub fn newline(source: &str, cursor: usize) -> Option<Edit> {
    lines::newline(source, cursor)
}

/// Tab (or Shift-Tab with `outdent`) on `selection`: move the list items it touches by the
/// width of their marker. `None` if nothing changes.
pub fn indent(source: &str, selection: Range<usize>, outdent: bool) -> Option<Edit> {
    let sel = clamp(source, selection);
    lines::indent(source, sel, outdent)
}

/// Sort, clamp to the text and move onto char boundaries (outwards).
pub(crate) fn clamp(source: &str, sel: Range<usize>) -> Range<usize> {
    let (a, b) = if sel.start <= sel.end {
        (sel.start, sel.end)
    } else {
        (sel.end, sel.start)
    };
    let mut a = a.min(source.len());
    let mut b = b.min(source.len());
    let point = a == b;
    while !source.is_char_boundary(a) {
        a -= 1;
    }
    if point {
        // A point stays a point.
        return a..a;
    }
    while !source.is_char_boundary(b) {
        b += 1;
    }
    a..b
}

/// The run of alphanumeric chars around `at`, if any.
fn word_at(source: &str, at: usize) -> Option<Range<usize>> {
    let start = source[..at]
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_alphanumeric())
        .last()
        .map_or(at, |(i, _)| i);
    let end = source[at..]
        .char_indices()
        .find(|(_, c)| !c.is_alphanumeric())
        .map_or(source.len(), |(i, _)| at + i);
    (start < end).then_some(start..end)
}

/// Is a run of `n` `*` an emphasis mark of this width? A run of exactly 2 is strong;
/// 1 or 3 and more hold emphasis.
fn run_ok(delim: &str, n: usize) -> bool {
    match delim {
        "*" => n == 1 || n >= 3,
        "**" => n >= 2,
        _ => true,
    }
}

/// The length of the run of `*` at the end (or start) of `s`.
fn star_run(s: &str, from_end: bool) -> usize {
    if from_end {
        s.chars().rev().take_while(|&c| c == '*').count()
    } else {
        s.chars().take_while(|&c| c == '*').count()
    }
}

/// Can `text` be the inside of one span of `delim`? It must not touch whitespace at its
/// ends (CommonMark's flanking rule) and must not hold the delimiter itself, or the marks
/// beside it belong to two neighbouring spans. Runs of `*` at its ends are other marks.
fn is_inside(text: &str, delim: &str) -> bool {
    let core = if delim.starts_with('*') {
        text.trim_matches('*')
    } else {
        text
    };
    let flush = |c: Option<char>| c.is_none_or(|c| !c.is_whitespace());
    flush(text.chars().next()) && flush(text.chars().next_back()) && !core.contains(delim)
}

/// The `[text](url)` around `sel`, taken off: replaced by its text, which is selected.
fn unlink(source: &str, sel: &Range<usize>) -> Option<Edit> {
    let mut from = 0;
    while let Some(i) = source[from..].find('[').map(|i| from + i) {
        from = i + 1;
        let rest = &source[i + 1..];
        let Some(mid) = rest.find("](") else { continue };
        let text = &rest[..mid];
        let url_rest = &rest[mid + 2..];
        let Some(close) = url_rest.find(')') else {
            continue;
        };
        if text.contains(['[', ']', '\n']) || url_rest[..close].contains(['\n', ' ']) {
            continue;
        }
        let end = i + 1 + mid + 2 + close + 1;
        if sel.start >= i && sel.end <= end {
            return Some(Edit {
                replace: i..end,
                with: text.to_string(),
                selection: i..i + text.len(),
            });
        }
    }
    None
}

fn toggle(source: &str, sel: Range<usize>, delim: &str) -> Edit {
    let d = delim.len();
    let star = delim.starts_with('*');
    let ok = |before: usize, after: usize| !star || (run_ok(delim, before) && run_ok(delim, after));

    // The cursor (if any) to keep inside the text, and the range to mark.
    let (cursor, range) = if sel.is_empty() {
        match word_at(source, sel.start) {
            Some(w) => (Some(sel.start), w),
            None => {
                // Inside an empty pair already? Remove it.
                let (s, e) = (sel.start, sel.start);
                if source[..s].ends_with(delim) && source[e..].starts_with(delim) {
                    let at = s - d;
                    return Edit {
                        replace: at..e + d,
                        with: String::new(),
                        selection: at..at,
                    };
                }
                return Edit {
                    replace: s..e,
                    with: format!("{delim}{delim}"),
                    selection: s + d..s + d,
                };
            }
        }
    } else {
        (None, sel)
    };
    let (start, end) = (range.start, range.end);
    let text = &source[start..end];

    // Delimiters at the edges of the selection.
    if text.len() >= 2 * d
        && text.starts_with(delim)
        && text.ends_with(delim)
        && ok(star_run(text, false), star_run(text, true))
        && is_inside(&text[d..text.len() - d], delim)
    {
        let inner = &text[d..text.len() - d];
        return Edit {
            replace: start..end,
            with: inner.to_string(),
            selection: match cursor {
                Some(c) => {
                    let c = c.saturating_sub(d).max(start);
                    c..c
                }
                None => start..start + inner.len(),
            },
        };
    }
    // Delimiters just outside.
    let (before, after) = (&source[..start], &source[end..]);
    if before.ends_with(delim)
        && after.starts_with(delim)
        && ok(star_run(before, true), star_run(after, false))
        && is_inside(text, delim)
    {
        let at = start - d;
        return Edit {
            replace: at..end + d,
            with: text.to_string(),
            selection: match cursor {
                Some(c) => c - d..c - d,
                None => at..at + text.len(),
            },
        };
    }
    // Wrap.
    Edit {
        replace: start..end,
        with: format!("{delim}{text}{delim}"),
        selection: match cursor {
            Some(c) => c + d..c + d,
            None => start + d..end + d,
        },
    }
}

fn link(source: &str, sel: Range<usize>) -> Edit {
    let range = if sel.is_empty() {
        word_at(source, sel.start).unwrap_or(sel)
    } else {
        sel
    };
    let text = &source[range.clone()];
    let url_start = range.start + 1 + text.len() + 2;
    Edit {
        replace: range,
        with: format!("[{text}](url)"),
        selection: url_start..url_start + 3,
    }
}

mod lines;

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str, sel: Range<usize>, cmd: Command) -> (String, Range<usize>) {
        let edit = command(src, sel, cmd).expect("an edit");
        let mut text = src.to_string();
        edit.apply(&mut text);
        (text, edit.selection)
    }

    #[test]
    fn strong_wraps_selection() {
        assert_eq!(
            run("a b c", 2..3, Command::Strong),
            ("a **b** c".into(), 4..5)
        );
    }

    #[test]
    fn strong_unwraps() {
        assert_eq!(
            run("a **b** c", 4..5, Command::Strong),
            ("a b c".into(), 2..3)
        );
        assert_eq!(
            run("a **b** c", 2..7, Command::Strong),
            ("a b c".into(), 2..3)
        );
    }

    #[test]
    fn word_at_cursor() {
        assert_eq!(
            run("hello world", 8..8, Command::Emphasis),
            ("hello *world*".into(), 9..9)
        );
    }

    #[test]
    fn empty_cursor_inserts_pair() {
        assert_eq!(run("a  b", 2..2, Command::Code), ("a `` b".into(), 3..3));
    }

    #[test]
    fn link() {
        assert_eq!(
            run("see this", 4..8, Command::Link),
            ("see [this](url)".into(), 11..14)
        );
    }

    #[test]
    fn japanese_and_emoji() {
        assert_eq!(
            run("日本語🎌です", 3..13, Command::Strong),
            ("日**本語🎌**です".into(), 5..15)
        );
    }

    #[test]
    fn cursor_in_marked_word_unwraps() {
        assert_eq!(
            run("hello *world*", 9..9, Command::Emphasis),
            ("hello world".into(), 8..8)
        );
    }

    #[test]
    fn emphasis_and_strong_runs() {
        assert_eq!(
            run("***a***", 3..4, Command::Emphasis),
            ("**a**".into(), 2..3)
        );
        assert_eq!(
            run("**a**", 2..3, Command::Emphasis),
            ("***a***".into(), 3..4)
        );
        assert_eq!(run("***a***", 3..4, Command::Strong), ("*a*".into(), 1..2));
        assert_eq!(run("*a*", 1..2, Command::Strong), ("***a***".into(), 3..4));
    }

    #[test]
    fn two_spans_are_not_one_pair() {
        for (src, cmd, want) in [
            ("**a** **b**", Command::Strong, "****a** **b****"),
            ("`a` `b`", Command::Code, "``a` `b``"),
            ("~~a~~ ~~b~~", Command::Strikethrough, "~~~~a~~ ~~b~~~~"),
        ] {
            let (text, _) = run(src, 0..src.len(), cmd);
            assert_eq!(text, want);
        }
    }

    #[test]
    fn neighbouring_spans_are_not_unwrapped() {
        assert_eq!(run("a* b *c", 2..5, Command::Emphasis).0, "a** b **c");
        assert_eq!(
            run("**a** b **c**", 5..8, Command::Strong).0,
            "**a**** b ****c**"
        );
    }

    #[test]
    fn link_toggles_off() {
        let src = "see [this](url) ok";
        assert_eq!(run(src, 4..15, Command::Link), ("see this ok".into(), 4..8));
        assert_eq!(run(src, 6..6, Command::Link), ("see this ok".into(), 4..8));
        assert_eq!(
            run(src, 12..12, Command::Link),
            ("see this ok".into(), 4..8)
        );
    }

    #[test]
    fn empty_selection_in_a_char_stays_a_point() {
        assert_eq!(run("日", 1..1, Command::Code), ("`日`".into(), 1..1));
        assert_eq!(command("日", 1..1, Command::Link).unwrap().replace, 0..3);
    }

    #[test]
    fn odd_selections_do_not_panic() {
        // Inside the 3-byte char, reversed, and past the end.
        let src = "日本";
        let reversed = Range { start: 5, end: 1 };
        for sel in [1..2, reversed, 4..40, 40..50, 0..0, 6..6] {
            for cmd in [
                Command::Strong,
                Command::Emphasis,
                Command::Strikethrough,
                Command::Code,
                Command::Link,
            ] {
                let (text, selection) = run(src, sel.clone(), cmd);
                assert!(text.is_char_boundary(selection.start));
                assert!(text.is_char_boundary(selection.end));
            }
        }
        assert_eq!(run("", 0..0, Command::Strong), ("****".into(), 2..2));
    }

    #[test]
    fn heading_levels() {
        let h = Command::Heading;
        assert_eq!(run("a", 0..0, h(2)).0, "## a");
        assert_eq!(run("# a", 0..0, h(2)).0, "## a");
        assert_eq!(run("## a", 0..0, h(2)).0, "a");
        assert_eq!(run("## a", 0..0, h(0)).0, "a");
        assert_eq!(run("  # a\nb", 0..7, h(3)).0, "  ### a\n### b");
        assert_eq!(run("## a", 0..4, h(2)), ("a".into(), 0..1));
    }

    #[test]
    fn lists_over_lines() {
        let (t, sel) = run("a\nb", 0..3, Command::BulletList);
        assert_eq!((t.as_str(), sel.clone()), ("- a\n- b", 2..7));
        assert_eq!(run(&t, sel, Command::BulletList), ("a\nb".into(), 0..3));
        assert_eq!(run("a\nb", 0..3, Command::NumberedList).0, "1. a\n2. b");
        assert_eq!(run("a\nb", 0..3, Command::TaskList).0, "- [ ] a\n- [ ] b");
        assert_eq!(run("1. a", 0..0, Command::BulletList).0, "- a");
        assert_eq!(run("- a", 0..0, Command::NumberedList).0, "1. a");
        assert_eq!(run("- [ ] a", 0..0, Command::BulletList).0, "- a");
        assert_eq!(run("- a", 0..0, Command::TaskList).0, "- [ ] a");
        assert_eq!(run("- [x] a", 0..0, Command::TaskList).0, "a");
        // Mixed: add to the lacking line; nesting and blank lines are kept.
        assert_eq!(
            run("- a\n  b\n\nc", 0..12, Command::BulletList).0,
            "- a\n  - b\n\n- c"
        );
        assert_eq!(run("a\n\nb", 0..5, Command::NumberedList).0, "1. a\n\n2. b");
    }

    #[test]
    fn selection_ending_at_a_line_start_skips_that_line() {
        assert_eq!(
            run("a\nb", 0..2, Command::BulletList),
            ("- a\nb".into(), 2..4)
        );
        assert_eq!(
            run("a\nb", 2..2, Command::BulletList),
            ("a\n- b".into(), 4..4)
        );
    }

    #[test]
    fn quote() {
        let (t, sel) = run("a\nb", 0..3, Command::Quote);
        assert_eq!((t.as_str(), sel.clone()), ("> a\n> b", 2..7));
        assert_eq!(run(&t, sel, Command::Quote), ("a\nb".into(), 0..3));
        assert_eq!(run("a\n\nb", 0..5, Command::Quote).0, "> a\n>\n> b");
        assert_eq!(run("> a\n>\n> b", 0..9, Command::Quote).0, "a\n\nb");
    }

    #[test]
    fn code_block() {
        let (t, sel) = run("a\nb", 0..3, Command::CodeBlock);
        assert_eq!((t.as_str(), sel.clone()), ("```\na\nb\n```", 4..7));
        assert_eq!(run(&t, sel, Command::CodeBlock), ("a\nb".into(), 0..3));
        assert_eq!(run(&t, 0..t.len(), Command::CodeBlock).0, "a\nb");
        assert_eq!(
            run("x\n```\na\n```\ny", 6..7, Command::CodeBlock).0,
            "x\na\ny"
        );
        assert_eq!(run("a\n", 0..0, Command::CodeBlock).0, "```\na\n```\n");
    }

    #[test]
    fn ends() {
        let all = [
            Command::Heading(1),
            Command::Heading(0),
            Command::BulletList,
            Command::NumberedList,
            Command::TaskList,
            Command::Quote,
            Command::CodeBlock,
        ];
        for cmd in all {
            for (src, sel) in [("", 0..0), ("a\nb", 3..3), ("a\nb", 0..0), ("a\n", 2..2)] {
                // Nothing to do is None (removing a heading that is not there).
                let Some(edit) = command(src, sel, cmd) else {
                    assert_eq!(cmd, Command::Heading(0));
                    continue;
                };
                let mut text = src.to_string();
                edit.apply(&mut text);
                let selection = edit.selection;
                assert!(selection.start <= selection.end && selection.end <= text.len());
                assert!(text.is_char_boundary(selection.start));
                assert!(text.is_char_boundary(selection.end));
            }
        }
        assert_eq!(run("", 0..0, Command::BulletList), ("- ".into(), 2..2));
        assert_eq!(
            run("", 0..0, Command::CodeBlock),
            ("```\n\n```".into(), 4..4)
        );
        assert_eq!(run("a\nb", 3..3, Command::Quote), ("a\n> b".into(), 5..5));
    }

    #[test]
    fn crlf_is_kept() {
        assert_eq!(run("a\r\nb", 0..4, Command::BulletList).0, "- a\r\n- b");
        assert_eq!(run("- a\r\n- b", 0..8, Command::BulletList).0, "a\r\nb");
        assert_eq!(
            run("a\r\nb", 0..4, Command::CodeBlock).0,
            "```\r\na\r\nb\r\n```"
        );
        assert_eq!(run("```\r\na\r\n```", 5..5, Command::CodeBlock).0, "a");
        assert_eq!(run("a\r\n\r\nb", 0..6, Command::Quote).0, "> a\r\n>\r\n> b");
    }

    #[test]
    fn nothing_to_do_is_none() {
        assert_eq!(command("a", 0..0, Command::Heading(0)), None);
        assert_eq!(command("", 0..0, Command::Heading(0)), None);
    }

    #[test]
    fn code_block_over_part_of_a_block_is_none() {
        let src = "```\na\n```\nb\n```\nc\n```";
        // Inside to outside, outside to inside, and across two blocks.
        for sel in [4..11, 8..14, 0..src.len(), 0..12] {
            assert_eq!(command(src, sel, Command::CodeBlock), None);
        }
    }

    fn nl(src: &str, cursor: usize) -> Option<(String, Range<usize>)> {
        let edit = newline(src, cursor)?;
        let mut text = src.to_string();
        edit.apply(&mut text);
        Some((text, edit.selection))
    }

    fn ind(src: &str, sel: Range<usize>, out: bool) -> Option<String> {
        let edit = indent(src, sel, out)?;
        let mut text = src.to_string();
        edit.apply(&mut text);
        Some(text)
    }

    #[test]
    fn continues() {
        assert_eq!(nl("- a", 3), Some(("- a\n- ".into(), 6..6)));
        assert_eq!(nl("3. a", 4), Some(("3. a\n4. ".into(), 8..8)));
        assert_eq!(nl("1) a", 4).unwrap().0, "1) a\n2) ");
        assert_eq!(nl("- [x] a", 7).unwrap().0, "- [x] a\n- [ ] ");
        assert_eq!(nl("  - a", 5).unwrap().0, "  - a\n  - ");
        assert_eq!(nl("> - a", 5).unwrap().0, "> - a\n> - ");
    }

    #[test]
    fn empty_item_leaves() {
        assert_eq!(nl("- a\n- ", 6), Some(("- a\n".into(), 4..4)));
        assert_eq!(nl("- ", 2), Some((String::new(), 0..0)));
        assert_eq!(nl("> - a\n> - ", 10).unwrap().0, "> - a\n> ");
        // A nested empty item leaves the list, indent and all.
        assert_eq!(nl("- a\n  - ", 8), Some(("- a\n".into(), 4..4)));
    }

    #[test]
    fn not_in_list() {
        assert_eq!(nl("a", 1), None);
        assert_eq!(nl("", 0), None);
        assert_eq!(nl("```\n- a\n```", 7), None);
        assert_eq!(nl("- a", 1), None);
    }

    #[test]
    fn mid_item_splits() {
        assert_eq!(nl("- ab", 3), Some(("- a\n- b".into(), 6..6)));
    }

    #[test]
    fn crlf() {
        assert_eq!(nl("- a\r\n- b", 3).unwrap().0, "- a\r\n- \r\n- b");
    }

    #[test]
    fn indents() {
        assert_eq!(ind("- a\n- b", 0..7, false).unwrap(), "  - a\n  - b");
        assert_eq!(ind("  - a\n  - b", 0..11, true).unwrap(), "- a\n- b");
        assert_eq!(ind("1. a", 0..4, false).unwrap(), "   1. a");
        assert_eq!(ind("- [ ] a", 0..0, false).unwrap(), "  - [ ] a");
        assert_eq!(ind("- a\nb", 0..5, false).unwrap(), "  - a\nb");
        assert_eq!(ind("> - a", 0..5, false).unwrap(), ">   - a");
    }

    #[test]
    fn outdent_at_zero() {
        assert_eq!(ind("- a", 0..3, true), None);
        assert_eq!(ind("a", 0..1, false), None);
        assert_eq!(ind(" - a", 0..4, true).unwrap(), "- a");
    }
}
