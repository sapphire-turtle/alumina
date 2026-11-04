//! The line commands: headings, lists, quotes and code blocks.
//!
//! Each works out the lines the selection touches, then a list of splices (small
//! replacements at the start of lines), applies them and maps the selection through them.

use super::{Command, Edit};
use std::ops::Range;

/// One line of the source: `start..end` is its text without the line break, `next` is
/// where the following line starts (the end of the source for the last line).
#[derive(Clone, Copy)]
struct Line {
    start: usize,
    end: usize,
    next: usize,
}

/// A replacement inside the source. A selection edge at `at` moves past `ins` if `after`.
struct Splice {
    at: usize,
    del: usize,
    ins: String,
    after: bool,
}

impl Splice {
    fn new(at: usize, del: usize, ins: impl Into<String>, after: bool) -> Self {
        Self {
            at,
            del,
            ins: ins.into(),
            after,
        }
    }
}

fn split(source: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    loop {
        match source[start..].find('\n') {
            Some(i) => {
                let nl = start + i;
                let end = if nl > start && source.as_bytes()[nl - 1] == b'\r' {
                    nl - 1
                } else {
                    nl
                };
                lines.push(Line {
                    start,
                    end,
                    next: nl + 1,
                });
                start = nl + 1;
            }
            None => {
                lines.push(Line {
                    start,
                    end: source.len(),
                    next: source.len(),
                });
                return lines;
            }
        }
    }
}

fn indent_len(text: &str) -> usize {
    text.len() - text.trim_start_matches([' ', '\t']).len()
}

fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Numbered,
    Task,
}

/// The list marker after the indentation: its kind and its length with the spaces after.
fn list_marker(text: &str) -> Option<(ListKind, usize)> {
    let ind = indent_len(text);
    let rest = &text[ind..];
    let spaces = |s: &str| s.len() - s.trim_start_matches([' ', '\t']).len();
    let boundary = |s: &str| s.is_empty() || s.starts_with([' ', '\t']);
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=9).contains(&digits) {
        let after = &rest[digits..];
        if after.starts_with(['.', ')']) && boundary(&after[1..]) {
            return Some((ListKind::Numbered, digits + 1 + spaces(&after[1..])));
        }
        return None;
    }
    if rest.starts_with(['-', '*', '+']) && boundary(&rest[1..]) {
        let after = &rest[1..];
        let sp = spaces(after);
        let box_ = &after[sp..];
        if sp > 0
            && box_.len() >= 3
            && box_.starts_with('[')
            && box_[1..].starts_with([' ', 'x', 'X'])
            && box_[2..].starts_with(']')
            && boundary(&box_[3..])
        {
            return Some((ListKind::Task, 1 + sp + 3 + spaces(&box_[3..])));
        }
        return Some((ListKind::Bullet, 1 + sp));
    }
    None
}

/// The `#` marks after the indentation: the level and the length with the spaces after.
fn heading(text: &str) -> Option<(usize, usize)> {
    let ind = indent_len(text);
    let rest = &text[ind..];
    let level = rest.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let after = &rest[level..];
    if !after.is_empty() && !after.starts_with([' ', '\t']) {
        return None;
    }
    Some((level, level + indent_len(after)))
}

/// The length of the `>` mark (with one space after) after the indentation.
fn quote_mark(text: &str) -> Option<usize> {
    let rest = &text[indent_len(text)..];
    rest.starts_with('>')
        .then(|| if rest[1..].starts_with(' ') { 2 } else { 1 })
}

/// The fenced blocks (as line indices of the opening and closing fences).
fn fences(source: &str, lines: &[Line]) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, char, usize)> = None;
    for (i, l) in lines.iter().enumerate() {
        let text = source[l.start..l.end].trim_start_matches(' ');
        let Some(c) = text.chars().next().filter(|c| matches!(c, '`' | '~')) else {
            continue;
        };
        let run = text.chars().take_while(|&x| x == c).count();
        if run < 3 {
            continue;
        }
        let tail = &text[run..];
        match open {
            None => open = Some((i, c, run)),
            Some((o, oc, orun)) if oc == c && run >= orun && tail.trim().is_empty() => {
                blocks.push((o, i));
                open = None;
            }
            _ => {}
        }
    }
    blocks
}

pub(super) fn edit(source: &str, sel: Range<usize>, command: Command) -> Option<Edit> {
    let eol = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let lines = split(source);
    let line_of = |p: usize| lines.partition_point(|l| l.start <= p) - 1;
    let first = line_of(sel.start);
    let mut last = line_of(sel.end);
    if sel.start < sel.end && last > first && lines[last].start == sel.end {
        last -= 1;
    }
    let text = |i: usize| &source[lines[i].start..lines[i].end];
    let touched = first..=last;
    // Blank lines are left alone, unless every touched line is blank.
    let any_text = touched.clone().any(|i| !is_blank(text(i)));
    let active: Vec<usize> = touched
        .clone()
        .filter(|&i| !any_text || !is_blank(text(i)))
        .collect();

    let mut splices = Vec::new();
    match command {
        Command::Heading(n) => {
            let n = usize::from(n.min(6));
            let all = n == 0
                || active
                    .iter()
                    .all(|&i| heading(text(i)).map(|h| h.0) == Some(n));
            for &i in &active {
                let t = text(i);
                let at = lines[i].start + indent_len(t);
                let del = heading(t).map_or(0, |h| h.1);
                let ins = if all || n == 0 {
                    String::new()
                } else {
                    format!("{} ", "#".repeat(n))
                };
                if del > 0 || !ins.is_empty() {
                    splices.push(Splice::new(at, del, ins, true));
                }
            }
        }
        Command::BulletList | Command::NumberedList | Command::TaskList => {
            let target = match command {
                Command::BulletList => ListKind::Bullet,
                Command::NumberedList => ListKind::Numbered,
                _ => ListKind::Task,
            };
            let kind = |i: usize| list_marker(text(i)).map(|m| m.0);
            let all = active.iter().all(|&i| kind(i) == Some(target));
            let mut number = 1;
            for &i in &active {
                let t = text(i);
                let at = lines[i].start + indent_len(t);
                let marker = list_marker(t);
                let del = marker.map_or(0, |m| m.1);
                if all {
                    splices.push(Splice::new(at, del, "", true));
                    continue;
                }
                let ins = match target {
                    ListKind::Numbered => {
                        number += 1;
                        format!("{}. ", number - 1)
                    }
                    _ if kind(i) == Some(target) => continue,
                    ListKind::Bullet => "- ".to_string(),
                    ListKind::Task => "- [ ] ".to_string(),
                };
                if source[at..at + del] != ins {
                    splices.push(Splice::new(at, del, ins, true));
                }
            }
        }
        Command::Quote => {
            let all = active.iter().all(|&i| quote_mark(text(i)).is_some());
            let (lo, hi) = (active.first().copied(), active.last().copied());
            for &i in &touched.clone().collect::<Vec<_>>() {
                let t = text(i);
                let at = lines[i].start + indent_len(t);
                let mark = quote_mark(t);
                if all {
                    if let Some(m) = mark {
                        splices.push(Splice::new(at, m, "", true));
                    }
                } else if mark.is_none() {
                    if is_blank(t) && any_text {
                        // Between quoted lines, a bare mark keeps one quote.
                        if lo.is_some_and(|lo| i > lo) && hi.is_some_and(|hi| i < hi) {
                            splices.push(Splice::new(lines[i].end, 0, ">", true));
                        }
                    } else {
                        splices.push(Splice::new(at, 0, "> ", true));
                    }
                }
            }
        }
        Command::CodeBlock => {
            let blocks = fences(source, &lines);
            let block = blocks
                .iter()
                .copied()
                .find(|&(o, c)| first >= o && last <= c);
            // Partly in a block, or around one: wrapping would nest fences.
            if block.is_none() && blocks.iter().any(|&(o, c)| first <= c && last >= o) {
                return None;
            }
            if let Some((o, c)) = block {
                let (open, close) = (lines[o], lines[c]);
                if c == o + 1 && close.next == close.end {
                    splices.push(Splice::new(open.start, close.end - open.start, "", true));
                } else {
                    splices.push(Splice::new(open.start, open.next - open.start, "", true));
                    if close.next > close.end {
                        splices.push(Splice::new(close.start, close.next - close.start, "", true));
                    } else {
                        let prev = lines[c - 1].end;
                        splices.push(Splice::new(prev, close.end - prev, "", true));
                    }
                }
            } else {
                splices.push(Splice::new(
                    lines[first].start,
                    0,
                    format!("```{eol}"),
                    true,
                ));
                splices.push(Splice::new(lines[last].end, 0, format!("{eol}```"), false));
            }
        }
        // The inline commands are not line commands.
        _ => {}
    }
    finish(source, &sel, eol, splices)
}

/// Apply the splices (in order, not overlapping) and map the selection through them.
fn finish(source: &str, sel: &Range<usize>, eol: &str, mut splices: Vec<Splice>) -> Option<Edit> {
    // Line breaks in inserted text follow the source.
    for s in &mut splices {
        if eol == "\r\n" {
            s.ins = s.ins.replace("\r\n", "\n").replace('\n', "\r\n");
        }
    }
    splices.sort_by_key(|s| s.at);
    let lo = splices.first()?.at;
    let hi = splices.iter().map(|s| s.at + s.del).max().unwrap_or(lo);
    let mut with = String::new();
    let mut at = lo;
    for s in &splices {
        with.push_str(&source[at..s.at]);
        with.push_str(&s.ins);
        at = s.at + s.del;
    }
    with.push_str(&source[at..hi]);

    let map = |p: usize| {
        let mut shift = 0isize;
        for s in &splices {
            if p < s.at || (p == s.at && !s.after) {
                break;
            }
            if p >= s.at + s.del {
                shift += s.ins.len() as isize - s.del as isize;
            } else {
                return (s.at + s.ins.len()).saturating_add_signed(shift);
            }
        }
        p.saturating_add_signed(shift)
    };
    Some(Edit {
        replace: lo..hi,
        with,
        selection: map(sel.start)..map(sel.end),
    })
}

/// The length of the quote marks (`> `, nested too) and indentation that start `text`.
fn container_prefix(text: &str) -> usize {
    let mut at = 0;
    loop {
        let rest = &text[at..];
        let ind = indent_len(rest);
        match quote_mark(rest) {
            Some(m) => at += ind + m,
            None => return at,
        }
    }
}

/// Is the line whose list marker starts at `at` in a list item, and not in a code block?
fn in_list_item(source: &str, at: usize) -> bool {
    use crate::{BlockKind, parse};
    let doc = parse(source);
    let holds = |r: &Range<usize>| r.start <= at && at < r.end.max(r.start + 1);
    let code = doc
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::CodeBlock { .. }) && holds(&b.range));
    !code
        && doc
            .blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::ListItem { .. }) && holds(&b.range))
}

/// Enter at `cursor`: continue the list item the cursor is in, or leave it if it is empty.
pub(super) fn newline(source: &str, cursor: usize) -> Option<Edit> {
    let mut cursor = cursor.min(source.len());
    while !source.is_char_boundary(cursor) {
        cursor -= 1;
    }
    let eol = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let lines = split(source);
    let line = lines[lines.partition_point(|l| l.start <= cursor) - 1];
    let text = &source[line.start..line.end];
    let prefix = container_prefix(text);
    let (kind, len) = list_marker(&text[prefix..])?;
    let ind = prefix + indent_len(&text[prefix..]);
    let content = prefix + indent_len(&text[prefix..]) + len;
    let content = content.min(text.len());
    if cursor < line.start + content || cursor > line.end || !in_list_item(source, line.start + ind)
    {
        return None;
    }
    let marker = &text[ind..content];
    if is_blank(&text[content..]) && line.start + content <= cursor {
        // An empty item: take the marker off, keeping any quote.
        let keep = text[..prefix].trim_end_matches([' ', '\t']);
        let keep = if keep.is_empty() { "" } else { &text[..prefix] };
        return Some(Edit {
            replace: line.start..line.end,
            with: keep.to_string(),
            selection: line.start + keep.len()..line.start + keep.len(),
        });
    }
    let next = match kind {
        ListKind::Bullet => marker.to_string(),
        ListKind::Task => format!("{}[ ] ", &marker[..marker.find('[')?]),
        ListKind::Numbered => {
            let d = marker.bytes().take_while(u8::is_ascii_digit).count();
            let n: u64 = marker[..d].parse().ok()?;
            format!("{}{}", n + 1, &marker[d..])
        }
    };
    let with = format!("{eol}{}{next}", &text[..ind]);
    Some(Edit {
        replace: cursor..cursor,
        selection: cursor + with.len()..cursor + with.len(),
        with,
    })
}

/// Tab / Shift-Tab: move the list items touched by `sel` by the width of their marker.
pub(super) fn indent(source: &str, sel: Range<usize>, outdent: bool) -> Option<Edit> {
    let lines = split(source);
    let line_of = |p: usize| lines.partition_point(|l| l.start <= p) - 1;
    let first = line_of(sel.start);
    let mut last = line_of(sel.end);
    if sel.start < sel.end && last > first && lines[last].start == sel.end {
        last -= 1;
    }
    let mut splices = Vec::new();
    for l in &lines[first..=last] {
        let text = &source[l.start..l.end];
        let prefix = container_prefix(text);
        let rest = &text[prefix..];
        let Some((kind, len)) = list_marker(rest) else {
            continue;
        };
        if !in_list_item(source, l.start + prefix) {
            continue;
        }
        let ind = indent_len(rest);
        let marker = &rest[ind..];
        let width = match kind {
            ListKind::Task => marker.find('[').unwrap_or(len),
            _ => len,
        };
        let at = l.start + prefix;
        if outdent {
            let spaces = rest.bytes().take_while(|&b| b == b' ').count().min(width);
            if spaces > 0 {
                splices.push(Splice::new(at, spaces, "", true));
            }
        } else {
            splices.push(Splice::new(at, 0, " ".repeat(width), true));
        }
    }
    finish(source, &sel, "\n", splices)
}
