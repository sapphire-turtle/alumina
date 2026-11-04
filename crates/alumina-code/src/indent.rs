//! Edits on a source and a byte selection: each returns an [`Edit`] to apply.
//!
//! Everything here works in bytes on char boundaries; the editor converts egui's char
//! cursors at its edge.

use std::ops::Range;

/// The unit of indentation: one level is a tab or some spaces.
///
/// Columns are counted in chars, a tab as 4, which is how wide egui draws one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum Indent {
    /// One tab per level.
    Tab,
    /// This many spaces per level; `Spaces(0)` is taken as `Spaces(1)`.
    Spaces(u8),
}

impl Default for Indent {
    fn default() -> Self {
        Indent::Spaces(4)
    }
}

impl Indent {
    /// One level, as text.
    fn unit(self) -> String {
        match self {
            Indent::Tab => "\t".to_owned(),
            Indent::Spaces(n) => " ".repeat(usize::from(n.max(1))),
        }
    }

    /// The width of a level in columns, for stepping back to a multiple of it.
    fn width(self) -> usize {
        match self {
            Indent::Tab => TAB,
            Indent::Spaces(n) => usize::from(n.max(1)),
        }
    }
}

/// How wide a tab is, in columns.
const TAB: usize = 4;

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
    ///
    /// `text` must be the source the edit was made from (by [`newline`] or [`close`]): on
    /// any other text the range may be out of bounds or off a char boundary, and this
    /// panics.
    pub fn apply(&self, text: &mut String) {
        text.replace_range(self.replace.clone(), &self.with);
    }
}

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// The line break the source's first line ends with, `\n` if it has none.
fn line_break(source: &str) -> &'static str {
    match source.find('\n') {
        Some(i) if source[..i].ends_with('\r') => "\r\n",
        _ => "\n",
    }
}

/// The byte where the line holding `at` starts.
fn line_start(source: &str, at: usize) -> usize {
    source[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// Enter with `selection` selected: the edit that replaces it by a line break and the
/// indentation of its line, one level in after an opener, and with a closer after it (a
/// matching one) moved to a line of its own. A selection out of range, reversed or off a
/// char boundary is clamped.
pub fn newline(source: &str, selection: Range<usize>, indent: Indent) -> Edit {
    let sel = clamp(source, selection);
    let start = line_start(source, sel.start);
    let before = &source[start..sel.start];
    let base: String = before.chars().take_while(|&c| is_blank(c)).collect();
    let br = line_break(source);

    let opener = before.trim_end_matches(is_blank).chars().next_back();
    let closer = match opener {
        Some('{') => Some('}'),
        Some('[') => Some(']'),
        Some('(') => Some(')'),
        _ => None,
    };
    let after = &source[sel.end..];
    let rest = after.trim_start_matches(is_blank);
    let skipped = after.len() - rest.len();

    let mut with = String::from(br);
    with.push_str(&base);
    let Some(closer) = closer else {
        let cursor = sel.start + with.len();
        return Edit {
            replace: sel,
            with,
            selection: cursor..cursor,
        };
    };
    with.push_str(&indent.unit());
    let cursor = sel.start + with.len();
    let mut end = sel.end;
    if rest.starts_with(closer) {
        with.push_str(br);
        with.push_str(&base);
        end += skipped;
    }
    Edit {
        replace: sel.start..end,
        with,
        selection: cursor..cursor,
    }
}

/// A closing bracket (`}`, `]` or `)`) typed at `selection`: when nothing is selected and
/// everything before the cursor on its line is a level or more of spaces and tabs, the edit
/// that takes one level off the end of it before the bracket goes in. `None` otherwise, and
/// the editor lets egui type it.
pub fn close(source: &str, selection: Range<usize>, closer: char, indent: Indent) -> Option<Edit> {
    if !matches!(closer, '}' | ']' | ')') {
        return None;
    }
    let sel = clamp(source, selection);
    if sel.start != sel.end {
        return None;
    }
    let cursor = sel.start;
    let start = line_start(source, cursor);
    let before = &source[start..cursor];
    if before.is_empty() || !before.chars().all(is_blank) {
        return None;
    }
    let column: usize = before
        .chars()
        .map(|c| if c == '\t' { TAB } else { 1 })
        .sum();
    if column < indent.width() {
        return None;
    }
    let removed = if before.ends_with('\t') {
        1
    } else {
        let spaces = before.len() - before.trim_end_matches(' ').len();
        let previous = (column - 1) / indent.width() * indent.width();
        spaces.min(column - previous)
    };
    let from = cursor - removed;
    Some(Edit {
        replace: from..cursor,
        with: closer.to_string(),
        selection: from + 1..from + 1,
    })
}

/// One line of the source: where it starts, where its text ends (before a `\r`) and where
/// the line ends (before its `\n`).
struct Line {
    start: usize,
    text_end: usize,
    end: usize,
}

/// The lines the selection touches: from the line of its start to the line of its end,
/// leaving out the line of the end when that is at its start and the selection is not empty.
fn touched_lines(source: &str, sel: &Range<usize>) -> Vec<Line> {
    let first = line_start(source, sel.start);
    let at_start = sel.end > sel.start && source[..sel.end].ends_with('\n');
    let last = if at_start {
        line_start(source, sel.end - 1)
    } else {
        line_start(source, sel.end)
    };
    let mut lines = Vec::new();
    let mut start = first;
    loop {
        let end = source[start..]
            .find('\n')
            .map_or(source.len(), |i| start + i);
        let text_end = if source[start..end].ends_with('\r') {
            end - 1
        } else {
            end
        };
        lines.push(Line {
            start,
            text_end,
            end,
        });
        if start >= last {
            break;
        }
        start = end + 1;
    }
    lines
}

/// Tab with `selection` selected: over one line, the spaces (or tab) up to the next stop
/// replace the selection; over two lines or more, each line that is not blank goes a level
/// in. `None` when there is nothing to indent.
pub fn indent(source: &str, selection: Range<usize>, indent: Indent) -> Option<Edit> {
    let sel = clamp(source, selection);
    let lines = touched_lines(source, &sel);
    if lines.len() == 1 {
        let before = &source[lines[0].start..sel.start];
        let column: usize = before
            .chars()
            .map(|c| if c == '\t' { TAB } else { 1 })
            .sum();
        let with = match indent {
            Indent::Tab => "\t".to_owned(),
            Indent::Spaces(_) => " ".repeat(indent.width() - column % indent.width()),
        };
        let cursor = sel.start + with.len();
        return Some(Edit {
            replace: sel,
            with,
            selection: cursor..cursor,
        });
    }
    let unit = indent.unit();
    let indented: Vec<bool> = lines
        .iter()
        .map(|l| !source[l.start..l.text_end].chars().all(is_blank))
        .collect();
    if !indented.contains(&true) {
        return None;
    }
    let from = lines[0].start;
    let to = lines[lines.len() - 1].end;
    let mut with = String::new();
    for (i, (line, &go)) in lines.iter().zip(&indented).enumerate() {
        if i > 0 {
            with.push_str(&source[lines[i - 1].text_end..line.start]);
        }
        if go {
            with.push_str(&unit);
        }
        with.push_str(&source[line.start..line.text_end]);
    }
    with.push_str(&source[lines[lines.len() - 1].text_end..to]);
    let shift = |p: usize| {
        let added: usize = lines
            .iter()
            .zip(&indented)
            .filter(|(l, go)| **go && l.start < p)
            .count()
            * unit.len();
        p + added
    };
    Some(Edit {
        replace: from..to,
        with,
        selection: shift(sel.start)..shift(sel.end),
    })
}

/// How many bytes of indentation Shift+Tab takes off a line with this text.
fn outdent_len(text: &str, indent: Indent) -> usize {
    if text.starts_with('\t') {
        return 1;
    }
    let spaces = text.len() - text.trim_start_matches(' ').len();
    spaces.min(indent.width())
}

/// Shift+Tab with `selection` selected: the edit that takes a level off the start of each
/// line it touches. `None` when none of them has indentation.
pub fn outdent(source: &str, selection: Range<usize>, indent: Indent) -> Option<Edit> {
    let sel = clamp(source, selection);
    let lines = touched_lines(source, &sel);
    let removed: Vec<usize> = lines
        .iter()
        .map(|l| outdent_len(&source[l.start..l.text_end], indent))
        .collect();
    if removed.iter().all(|&r| r == 0) {
        return None;
    }
    let shift = |p: usize| {
        let mut p_new = p;
        for (l, &r) in lines.iter().zip(&removed) {
            if l.start < p {
                p_new -= r.min(p - l.start);
            }
        }
        p_new
    };
    let selection = shift(sel.start)..shift(sel.end);
    if lines.len() == 1 {
        let from = lines[0].start;
        return Some(Edit {
            replace: from..from + removed[0],
            with: String::new(),
            selection,
        });
    }
    let last = &lines[lines.len() - 1];
    let mut with = String::new();
    for (i, (line, &r)) in lines.iter().zip(&removed).enumerate() {
        if i > 0 {
            with.push_str(&source[lines[i - 1].text_end..line.start]);
        }
        with.push_str(&source[line.start + r..line.text_end]);
    }
    with.push_str(&source[last.text_end..last.end]);
    Some(Edit {
        replace: lines[0].start..last.end,
        with,
        selection,
    })
}

/// Backspace with a cursor in the spaces that start its line: the edit that takes it back
/// to the previous multiple of the level's width. `None` otherwise (and always for
/// [`Indent::Tab`]), and the editor lets egui delete.
pub fn backspace(source: &str, selection: Range<usize>, indent: Indent) -> Option<Edit> {
    if indent == Indent::Tab {
        return None;
    }
    let sel = clamp(source, selection);
    if sel.start != sel.end {
        return None;
    }
    let cursor = sel.start;
    let before = &source[line_start(source, cursor)..cursor];
    if before.is_empty() || !before.chars().all(|c| c == ' ') {
        return None;
    }
    let width = indent.width();
    let back = match before.len() % width {
        0 => width,
        n => n,
    };
    Some(Edit {
        replace: cursor - back..cursor,
        with: String::new(),
        selection: cursor - back..cursor - back,
    })
}

/// Sort, clamp to the text and move onto char boundaries (outwards).
fn clamp(source: &str, sel: Range<usize>) -> Range<usize> {
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

#[cfg(test)]
mod tests {
    use super::*;

    const S4: Indent = Indent::Spaces(4);

    fn ed(r: Range<usize>, w: &str, s: Range<usize>) -> Edit {
        Edit {
            replace: r,
            with: w.to_owned(),
            selection: s,
        }
    }

    #[test]
    fn enter_keeps_the_indentation() {
        assert_eq!(
            newline("    let x = 1;", 14..14, S4),
            ed(14..14, "\n    ", 19..19)
        );
    }
    #[test]
    fn enter_inside_the_indentation_keeps_what_is_before_the_cursor() {
        assert_eq!(newline("        x", 4..4, S4), ed(4..4, "\n    ", 9..9));
    }
    #[test]
    fn enter_after_an_opener_goes_in() {
        assert_eq!(newline("fn a() {", 8..8, S4), ed(8..8, "\n    ", 13..13));
        assert_eq!(newline("  x(  ", 6..6, S4), ed(6..6, "\n      ", 13..13)); // spaces after `(`
        assert_eq!(newline("[", 1..1, Indent::Tab), ed(1..1, "\n\t", 3..3));
    }
    #[test]
    fn enter_between_brackets_puts_the_closer_on_its_own_line() {
        assert_eq!(newline("a {}", 3..3, S4), ed(3..3, "\n    \n", 8..8));
        // The spaces between the cursor and the closer go.
        assert_eq!(
            newline("  x(  )", 4..4, S4),
            ed(4..6, "\n      \n  ", 11..11)
        );
        // Not a pair: only the level in.
        assert_eq!(newline("a {)", 3..3, S4), ed(3..3, "\n    ", 8..8));
    }
    #[test]
    fn enter_replaces_the_selection() {
        assert_eq!(newline("ab cd", 2..3, S4), ed(2..3, "\n", 3..3));
    }
    #[test]
    fn enter_counts_bytes_in_japanese() {
        let s = "  日本 {"; // 10 bytes
        assert_eq!(newline(s, 10..10, S4), ed(10..10, "\n      ", 17..17));
    }
    #[test]
    fn enter_matches_crlf() {
        assert_eq!(newline("x\r\n  y", 6..6, S4), ed(6..6, "\r\n  ", 10..10));
        assert_eq!(
            newline("a {}\r\n", 3..3, S4),
            ed(3..3, "\r\n    \r\n", 9..9)
        );
    }
    #[test]
    fn a_closer_on_a_blank_indented_line_comes_out_a_level() {
        assert_eq!(close("        ", 8..8, '}', S4), Some(ed(4..8, "}", 5..5)));
        assert_eq!(close("      ", 6..6, ']', S4), Some(ed(4..6, "]", 5..5))); // back to a multiple of 4
        assert_eq!(
            close("\t\t", 2..2, ')', Indent::Tab),
            Some(ed(1..2, ")", 2..2))
        );
        assert_eq!(
            close("    ", 4..4, ')', Indent::Tab),
            Some(ed(0..4, ")", 1..1))
        ); // up to 4 spaces
        assert_eq!(close("  \t", 3..3, '}', S4), Some(ed(2..3, "}", 3..3))); // a tab at the end goes
        assert_eq!(close("x\n    ", 6..6, '}', S4), Some(ed(2..6, "}", 3..3)));
    }
    #[test]
    fn a_closer_elsewhere_is_typed_by_egui() {
        assert_eq!(close("  x", 3..3, '}', S4), None);
        assert_eq!(close("", 0..0, '}', S4), None);
        assert_eq!(close("    ", 0..4, '}', S4), None);
        assert_eq!(close("    ", 4..4, 'x', S4), None);
        // Less than a level of indentation.
        assert_eq!(close(" ", 1..1, '}', S4), None);
        assert_eq!(close("  ", 2..2, '}', S4), None);
    }
    #[test]
    fn spaces_zero_is_one() {
        assert_eq!(
            close("  ", 2..2, '}', Indent::Spaces(0)),
            Some(ed(1..2, "}", 2..2))
        );
    }
    #[test]
    fn tab_inserts_up_to_the_next_stop() {
        assert_eq!(indent("ab", 1..1, S4), Some(ed(1..1, "   ", 4..4)));
        assert_eq!(indent("ab", 0..0, S4), Some(ed(0..0, "    ", 4..4)));
        assert_eq!(indent("日本", 6..6, S4), Some(ed(6..6, "  ", 8..8)));
        assert_eq!(indent("\tx", 2..2, S4), Some(ed(2..2, "   ", 5..5))); // column 5
        assert_eq!(indent("abc", 1..2, S4), Some(ed(1..2, "   ", 4..4)));
        assert_eq!(indent("ab", 1..1, Indent::Tab), Some(ed(1..1, "\t", 2..2)));
    }
    #[test]
    fn tab_over_lines_indents_each_but_the_blank() {
        assert_eq!(
            indent("a\nb\n\nc", 0..6, S4),
            Some(ed(0..6, "    a\n    b\n\n    c", 0..18))
        );
        assert_eq!(
            indent("a\n  \nb", 0..6, Indent::Tab),
            Some(ed(0..6, "\ta\n  \n\tb", 0..8))
        );
    }
    #[test]
    fn tab_leaves_a_line_the_selection_only_reaches() {
        assert_eq!(
            indent("a\nb\nc", 0..4, S4),
            Some(ed(0..3, "    a\n    b", 0..12))
        );
        assert_eq!(
            indent("a\nb\nc", 1..4, S4),
            Some(ed(0..3, "    a\n    b", 5..12))
        );
    }
    #[test]
    fn tab_over_blank_lines_only_does_nothing() {
        assert_eq!(indent("\n\n", 0..2, S4), None);
    }
    #[test]
    fn tab_over_crlf_lines() {
        assert_eq!(
            indent("a\r\nb", 0..4, S4),
            Some(ed(0..4, "    a\r\n    b", 0..12))
        );
    }
    #[test]
    fn shift_tab_takes_a_level_off_each_line() {
        assert_eq!(outdent("    a", 5..5, S4), Some(ed(0..4, "", 1..1)));
        assert_eq!(outdent("  a", 3..3, S4), Some(ed(0..2, "", 1..1)));
        assert_eq!(outdent("      a", 7..7, S4), Some(ed(0..4, "", 3..3)));
        assert_eq!(outdent("\ta", 2..2, S4), Some(ed(0..1, "", 1..1)));
        assert_eq!(
            outdent("      a", 7..7, Indent::Tab),
            Some(ed(0..4, "", 3..3))
        );
        assert_eq!(outdent("    a", 2..2, S4), Some(ed(0..4, "", 0..0))); // inside the indent
        assert_eq!(
            outdent("    a\n  b\nc", 0..11, S4),
            Some(ed(0..11, "a\nb\nc", 0..5))
        );
        assert_eq!(
            outdent("  日本\n  x", 2..11, S4),
            Some(ed(0..12, "日本\nx", 0..7))
        );
    }
    #[test]
    fn shift_tab_on_unindented_lines_does_nothing() {
        assert_eq!(outdent("a\nb", 0..3, S4), None);
    }
    #[test]
    fn backspace_in_the_indentation_takes_a_level() {
        assert_eq!(backspace("        ", 8..8, S4), Some(ed(4..8, "", 4..4)));
        assert_eq!(backspace("      x", 6..6, S4), Some(ed(4..6, "", 4..4)));
        assert_eq!(backspace("    x", 4..4, S4), Some(ed(0..4, "", 0..0)));
        assert_eq!(backspace(" ", 1..1, S4), Some(ed(0..1, "", 0..0)));
        assert_eq!(
            backspace("a\n      ", 8..8, Indent::Spaces(2)),
            Some(ed(6..8, "", 6..6))
        );
    }
    #[test]
    fn backspace_elsewhere_is_egui_s() {
        assert_eq!(backspace("  a  ", 5..5, S4), None);
        assert_eq!(backspace("\t  ", 3..3, S4), None);
        assert_eq!(backspace("    ", 4..4, Indent::Tab), None);
        assert_eq!(backspace("    ", 0..4, S4), None);
        assert_eq!(backspace("    ", 0..0, S4), None);
    }
    #[cfg(feature = "serde")]
    #[test]
    fn indent_reads_from_settings() {
        assert_eq!(serde_json::to_string(&Indent::Tab).unwrap(), r#""tab""#);
        assert_eq!(
            serde_json::to_string(&Indent::Spaces(2)).unwrap(),
            r#"{"spaces":2}"#
        );
        assert_eq!(
            serde_json::from_str::<Indent>(r#"{"spaces":2}"#).unwrap(),
            Indent::Spaces(2)
        );
        assert_eq!(Indent::default(), S4);
    }
}
