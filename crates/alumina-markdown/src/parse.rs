//! Source to [`Doc`]: the blocks of a Markdown text and where their marks are.
//!
//! Every range is a byte range of the source, on char boundaries. Marks (the `#`s, the
//! bullet, the `>`s, the fences) are never dropped from the text, only located.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

/// A parsed source: its blocks and inlines, in source order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Doc {
    /// Nested blocks come after their parent.
    pub blocks: Vec<Block>,
    /// Nested inlines come after their parent.
    pub inlines: Vec<Inline>,
}

/// A block of the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    /// The whole block, marks and trailing line ending included.
    pub range: Range<usize>,
    /// The block's own marks, in source order.
    pub marks: Vec<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockKind {
    Paragraph,
    /// `level` is 1 to 6.
    Heading {
        level: u8,
    },
    /// `depth` 0 is the top list. `task` is `Some(checked)` for `[ ]` and `[x]`.
    ListItem {
        depth: usize,
        number: Option<u64>,
        task: Option<bool>,
    },
    /// `depth` 1 is a single `>`.
    Quote {
        depth: usize,
    },
    /// `info` is `None` for an indented block or a bare fence.
    CodeBlock {
        info: Option<String>,
    },
    ThematicBreak,
}

/// An inline span of a paragraph or heading.
#[derive(Clone, Debug, PartialEq)]
pub struct Inline {
    pub kind: InlineKind,
    pub range: Range<usize>,
    pub marks: Vec<Range<usize>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InlineKind {
    Emphasis,
    Strong,
    Strikethrough,
    Code,
    Link { url: String, text: Range<usize> },
}

/// An open list or quote, for the depth of what is inside it.
enum Container {
    List { start: Option<u64>, index: u64 },
    Quote,
}

/// Reads `source` into its blocks.
pub fn parse(source: &str) -> Doc {
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut doc = Doc::default();
    let mut stack: Vec<Container> = Vec::new();
    // Open inlines: their index in `doc.inlines` and the span of what is inside.
    let mut autolinks: Vec<bool> = Vec::new();
    let mut open: Vec<(usize, Option<Range<usize>>)> = Vec::new();
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => push(&mut doc, BlockKind::Paragraph, range, Vec::new()),
                Tag::Heading { level, .. } => {
                    let marks = heading_marks(source, &range);
                    push(
                        &mut doc,
                        BlockKind::Heading { level: level as u8 },
                        range,
                        marks,
                    );
                }
                Tag::BlockQuote(_) => {
                    stack.push(Container::Quote);
                    let depth = stack
                        .iter()
                        .filter(|c| matches!(c, Container::Quote))
                        .count();
                    let marks = quote_marks(source, &range, depth);
                    push(&mut doc, BlockKind::Quote { depth }, range, marks);
                }
                Tag::List(start) => stack.push(Container::List { start, index: 0 }),
                Tag::Item => {
                    let depth = stack
                        .iter()
                        .filter(|c| matches!(c, Container::List { .. }))
                        .count()
                        .saturating_sub(1);
                    let mut number = None;
                    if let Some(Container::List { start, index }) = stack
                        .iter_mut()
                        .rev()
                        .find(|c| matches!(c, Container::List { .. }))
                    {
                        number = start.map(|s| s + *index);
                        *index += 1;
                    }
                    let marks = item_marks(source, &range);
                    push(
                        &mut doc,
                        BlockKind::ListItem {
                            depth,
                            number,
                            task: None,
                        },
                        range,
                        marks,
                    );
                }
                Tag::CodeBlock(kind) => {
                    let fenced = matches!(kind, CodeBlockKind::Fenced(_));
                    let info = match kind {
                        CodeBlockKind::Fenced(info) if !info.trim().is_empty() => {
                            Some(info.trim().to_string())
                        }
                        _ => None,
                    };
                    let marks = if fenced {
                        fence_marks(source, &range)
                    } else {
                        Vec::new()
                    };
                    push(&mut doc, BlockKind::CodeBlock { info }, range, marks);
                }
                Tag::Emphasis => open_inline(&mut doc, &mut open, InlineKind::Emphasis, range),
                Tag::Strong => open_inline(&mut doc, &mut open, InlineKind::Strong, range),
                Tag::Strikethrough => {
                    open_inline(&mut doc, &mut open, InlineKind::Strikethrough, range);
                }
                Tag::Link {
                    link_type,
                    dest_url,
                    ..
                } => {
                    let kind = InlineKind::Link {
                        url: dest_url.to_string(),
                        text: range.start..range.start,
                    };
                    open_inline(&mut doc, &mut open, kind, range);
                    // Keep the link type to tell `<url>` from `[text](url)`.
                    autolinks.push(matches!(link_type, LinkType::Autolink | LinkType::Email));
                }
                _ => {}
            },
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {
                close_inline(source, &mut doc, &mut open, None);
            }
            Event::End(TagEnd::Link) => {
                let auto = autolinks.pop().unwrap_or(false);
                close_inline(source, &mut doc, &mut open, Some(auto));
            }
            Event::Code(_) => {
                let marks = code_marks(source, &range);
                doc.inlines.push(Inline {
                    kind: InlineKind::Code,
                    range: range.clone(),
                    marks,
                });
                grow(&mut open, &range);
            }
            Event::Text(_) | Event::SoftBreak | Event::HardBreak => grow(&mut open, &range),
            Event::End(TagEnd::BlockQuote(_) | TagEnd::List(_)) => {
                stack.pop();
            }
            Event::TaskListMarker(checked) => {
                // The marker follows its item's start.
                if let Some(Block {
                    kind: BlockKind::ListItem { task, .. },
                    marks,
                    ..
                }) = doc
                    .blocks
                    .iter_mut()
                    .rev()
                    .find(|b| matches!(b.kind, BlockKind::ListItem { .. }))
                {
                    *task = Some(checked);
                    marks.push(range);
                }
            }
            Event::Rule => {
                let marks = trimmed_line(source, range.start, range.end)
                    .into_iter()
                    .collect();
                push(&mut doc, BlockKind::ThematicBreak, range, marks);
            }
            _ => {}
        }
    }
    doc
}

fn open_inline(
    doc: &mut Doc,
    open: &mut Vec<(usize, Option<Range<usize>>)>,
    kind: InlineKind,
    range: Range<usize>,
) {
    open.push((doc.inlines.len(), None));
    doc.inlines.push(Inline {
        kind,
        range,
        marks: Vec::new(),
    });
}

/// Widens every open inline's content span to `range`.
fn grow(open: &mut [(usize, Option<Range<usize>>)], range: &Range<usize>) {
    for (_, span) in open {
        *span = Some(match span.take() {
            Some(s) => s.start.min(range.start)..s.end.max(range.end),
            None => range.clone(),
        });
    }
}

/// Closes the innermost open inline and locates its marks. `autolink` is `Some` for a link.
fn close_inline(
    source: &str,
    doc: &mut Doc,
    open: &mut Vec<(usize, Option<Range<usize>>)>,
    autolink: Option<bool>,
) {
    let Some((index, span)) = open.pop() else {
        return;
    };
    let inline = &mut doc.inlines[index];
    let range = inline.range.clone();
    match (&mut inline.kind, autolink) {
        (InlineKind::Link { text, .. }, Some(auto)) => {
            let content = span.unwrap_or(range.start..range.start);
            if auto {
                *text = content;
                let (open, close) = (range.start, range.end);
                if source[range.clone()].starts_with('<') && source[range.clone()].ends_with('>') {
                    inline.marks = vec![open..open + 1, close - 1..close];
                }
            } else {
                let open_end = source[range.clone()]
                    .starts_with('[')
                    .then_some(range.start + 1);
                let text_end = content.end.max(open_end.unwrap_or(range.start));
                *text = match open_end {
                    Some(o) => content.start.max(o).min(text_end)..text_end,
                    None => content,
                };
                // The `]` after the text, then whatever closes the link.
                let close = source[text_end..range.end].find(']').map(|i| text_end + i);
                if let (Some(o), Some(c)) = (open_end, close) {
                    inline.marks = vec![range.start..o, c..range.end];
                }
            }
        }
        (kind, _) => {
            let (ch, need) = match kind {
                InlineKind::Strikethrough => ('~', 2),
                InlineKind::Strong => (source[range.clone()].chars().next().unwrap_or('*'), 2),
                _ => (source[range.clone()].chars().next().unwrap_or('*'), 1),
            };
            inline.marks = delimiter_marks(source, &range, ch, need);
        }
    }
}

/// The `need` chars of `ch` at each end of `range`, fewer if the run is shorter.
fn delimiter_marks(source: &str, range: &Range<usize>, ch: char, need: usize) -> Vec<Range<usize>> {
    let text = &source[range.clone()];
    let run = text.chars().take_while(|&c| c == ch).count().min(need);
    let tail = text
        .chars()
        .rev()
        .take_while(|&c| c == ch)
        .count()
        .min(need);
    let n = run.min(tail);
    // `ch` is ASCII, so a run of `n` is `n` bytes.
    if n == 0 || text.len() < 2 * n {
        return Vec::new();
    }
    vec![range.start..range.start + n, range.end - n..range.end]
}

fn code_marks(source: &str, range: &Range<usize>) -> Vec<Range<usize>> {
    let text = &source[range.clone()];
    let head = text.len() - text.trim_start_matches('`').len();
    let tail = text.len() - text.trim_end_matches('`').len();
    let n = head.min(tail);
    if n == 0 || text.len() < 2 * n {
        return Vec::new();
    }
    vec![range.start..range.start + n, range.end - n..range.end]
}

fn push(doc: &mut Doc, kind: BlockKind, range: Range<usize>, marks: Vec<Range<usize>>) {
    doc.blocks.push(Block { kind, range, marks });
}

/// `source[start..end]` without its container prefix (spaces, tabs, `>`) and trailing whitespace, or
/// `None` if nothing is left.
fn trimmed_line(source: &str, start: usize, end: usize) -> Option<Range<usize>> {
    let line = &source[start..end];
    let text = line.trim_end();
    let skipped = text.len() - text.trim_start_matches([' ', '\t', '>']).len();
    let (from, to) = (start + skipped, start + text.len());
    (from < to).then_some(from..to)
}

/// The byte ranges of the lines of `range`, line endings excluded (`\r` too), the first
/// starting at `range.start`.
fn lines(source: &str, range: &Range<usize>) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = range.start;
    while start < range.end {
        let rest = &source[start..range.end];
        let (len, next) = match rest.find('\n') {
            Some(i) => (i, start + i + 1),
            None => (rest.len(), range.end),
        };
        let line = &rest[..len];
        out.push(start..start + line.strip_suffix('\r').unwrap_or(line).len());
        start = next;
    }
    out
}

fn heading_marks(source: &str, range: &Range<usize>) -> Vec<Range<usize>> {
    let lines = lines(source, range);
    let Some(first) = lines.first() else {
        return Vec::new();
    };
    let text = &source[first.clone()];
    let indent = text.len() - text.trim_start_matches(' ').len();
    if text[indent..].starts_with('#') {
        let hashes = text[indent..].len() - text[indent..].trim_start_matches('#').len();
        let from = first.start + indent;
        let mut marks = Vec::new();
        marks.push(from..from + hashes);
        // A closing run of `#`s, set off from the text by a space.
        let line = text.trim_end();
        let rest = line.trim_end_matches('#');
        if rest.len() < line.len() && rest.len() > indent + hashes && rest.ends_with([' ', '\t']) {
            marks.push(first.start + rest.len()..first.start + line.len());
        }
        return marks;
    }
    // Setext: the underline is the last line.
    lines
        .last()
        .and_then(|l| trimmed_line(source, l.start, l.end))
        .into_iter()
        .collect()
}

fn item_marks(source: &str, range: &Range<usize>) -> Vec<Range<usize>> {
    let text = &source[range.clone()];
    let indent = text.len() - text.trim_start_matches([' ', '\t']).len();
    let rest = &text[indent..];
    let len = match rest.chars().next() {
        Some('-' | '+' | '*') => 1,
        Some(c) if c.is_ascii_digit() => {
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            digits + 1
        }
        _ => return Vec::new(),
    };
    std::iter::once(range.start + indent..range.start + indent + len).collect()
}

fn quote_marks(source: &str, range: &Range<usize>, depth: usize) -> Vec<Range<usize>> {
    let mut marks = Vec::new();
    for (i, line) in lines(source, range).into_iter().enumerate() {
        let text = &source[line.clone()];
        let mut pos = 0;
        // A quote that starts mid-line (inside another) is already at its own `>`.
        let outer = if i == 0 && range.start > 0 && !source[..range.start].ends_with('\n') {
            0
        } else {
            depth - 1
        };
        for n in 0..=outer {
            pos += text[pos..].len() - text[pos..].trim_start_matches([' ', '\t']).len();
            if !text[pos..].starts_with('>') {
                break;
            }
            if n == outer {
                marks.push(line.start + pos..line.start + pos + 1);
            }
            pos += 1;
        }
    }
    marks
}

fn fence_marks(source: &str, range: &Range<usize>) -> Vec<Range<usize>> {
    let lines = lines(source, range);
    let Some(first) = lines.first() else {
        return Vec::new();
    };
    let opening = source[first.clone()].trim_start_matches(' ');
    let Some(fence_char) = opening.chars().next().filter(|c| matches!(c, '`' | '~')) else {
        return Vec::new();
    };
    let run = opening.len() - opening.trim_start_matches(fence_char).len();
    let mut marks: Vec<Range<usize>> = trimmed_line(source, first.start, first.end)
        .into_iter()
        .collect();
    if let Some(last) = lines.last().filter(|_| lines.len() > 1) {
        // The line starts with its container's prefix: `>`s and indentation.
        let closing = trimmed_line(source, last.start, last.end);
        if let Some(closing) = closing {
            let text = &source[closing.clone()];
            if text.len() >= run && text.chars().all(|c| c == fence_char) {
                marks.push(closing);
            }
        }
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks<'a>(src: &'a str, b: &Block) -> Vec<&'a str> {
        b.marks.iter().map(|m| &src[m.clone()]).collect()
    }

    fn imarks<'a>(src: &'a str, i: &Inline) -> Vec<&'a str> {
        i.marks.iter().map(|m| &src[m.clone()]).collect()
    }

    const SAMPLE: &str = include_str!("../tests/sample.md");

    fn kinds(doc: &Doc) -> Vec<BlockKind> {
        doc.blocks.iter().map(|b| b.kind.clone()).collect()
    }

    #[test]
    fn atx_heading() {
        let src = "## Title\n";
        let doc = parse(src);
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].kind, BlockKind::Heading { level: 2 });
        assert_eq!(doc.blocks[0].range, 0..9);
        assert_eq!(marks(src, &doc.blocks[0]), ["##"]);
    }

    #[test]
    fn setext_heading() {
        let src = "Title\n===\n";
        let doc = parse(src);
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].kind, BlockKind::Heading { level: 1 });
        assert_eq!(marks(src, &doc.blocks[0]), ["==="]);
    }

    #[test]
    fn nested_lists() {
        let src = "- a\n  1. b\n- [x] c\n";
        let doc = parse(src);
        assert_eq!(
            kinds(&doc),
            [
                BlockKind::ListItem {
                    depth: 0,
                    number: None,
                    task: None
                },
                BlockKind::ListItem {
                    depth: 1,
                    number: Some(1),
                    task: None
                },
                BlockKind::ListItem {
                    depth: 0,
                    number: None,
                    task: Some(true)
                },
            ]
        );
        assert_eq!(marks(src, &doc.blocks[0]), ["-"]);
        assert_eq!(marks(src, &doc.blocks[1]), ["1."]);
        assert_eq!(marks(src, &doc.blocks[2]), ["-", "[x]"]);
    }

    #[test]
    fn quote_nested() {
        let src = "> a\n> > b\n";
        let doc = parse(src);
        assert_eq!(
            kinds(&doc)
                .into_iter()
                .filter(|k| matches!(k, BlockKind::Quote { .. }))
                .collect::<Vec<_>>(),
            [BlockKind::Quote { depth: 1 }, BlockKind::Quote { depth: 2 }]
        );
        let outer = doc
            .blocks
            .iter()
            .find(|b| b.kind == BlockKind::Quote { depth: 1 })
            .unwrap();
        let inner = doc
            .blocks
            .iter()
            .find(|b| b.kind == BlockKind::Quote { depth: 2 })
            .unwrap();
        assert_eq!(marks(src, outer), [">", ">"]);
        assert_eq!(marks(src, inner), [">"]);
        // The inner mark is on line 2, after the outer one.
        assert_eq!(inner.marks[0], 6..7);
    }

    #[test]
    fn fenced_code() {
        let src = "```rust\nlet a = 1;\n```\n";
        let doc = parse(src);
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(
            doc.blocks[0].kind,
            BlockKind::CodeBlock {
                info: Some("rust".into())
            }
        );
        assert_eq!(marks(src, &doc.blocks[0]), ["```rust", "```"]);
    }

    #[test]
    fn thematic_break() {
        let src = "a\n\n---\n";
        let doc = parse(src);
        assert_eq!(
            kinds(&doc),
            [BlockKind::Paragraph, BlockKind::ThematicBreak]
        );
        assert_eq!(marks(src, &doc.blocks[1]), ["---"]);
    }

    #[test]
    fn japanese_ranges() {
        let src = "# 見出し🎌\n- 項目\n";
        let doc = parse(src);
        assert_eq!(doc.blocks[0].range, 0..16);
        for b in &doc.blocks {
            for r in std::iter::once(&b.range).chain(&b.marks) {
                assert!(src.is_char_boundary(r.start) && src.is_char_boundary(r.end));
            }
        }
    }

    #[test]
    fn crlf() {
        let lf = "- a\n  1. b\n- [x] c\n";
        let crlf = lf.replace('\n', "\r\n");
        let (a, b) = (parse(lf), parse(&crlf));
        assert_eq!(kinds(&a), kinds(&b));
        for (x, y) in a.blocks.iter().zip(&b.blocks) {
            assert_eq!(marks(lf, x), marks(&crlf, y));
        }
    }

    fn only_code(doc: &Doc) -> &Block {
        doc.blocks
            .iter()
            .find(|b| matches!(b.kind, BlockKind::CodeBlock { .. }))
            .unwrap()
    }

    #[test]
    fn fence_in_quote() {
        let src = "> ```\n> x\n> ```\n";
        let doc = parse(src);
        assert_eq!(marks(src, only_code(&doc)), ["```", "```"]);
    }

    #[test]
    fn fence_in_list() {
        let src = "- ```\n  x\n  ```\n";
        let doc = parse(src);
        assert_eq!(marks(src, only_code(&doc)), ["```", "```"]);
        let src = "> - ```\n>   x\n>   ```\n";
        let doc = parse(src);
        assert_eq!(marks(src, only_code(&doc)), ["```", "```"]);
    }

    #[test]
    fn indented_code_has_no_marks() {
        let src = "    ```\n    x\n";
        let doc = parse(src);
        assert_eq!(doc.blocks[0].kind, BlockKind::CodeBlock { info: None });
        assert!(doc.blocks[0].marks.is_empty());
    }

    #[test]
    fn short_closing_fence_is_content() {
        let src = "````\nx\n```";
        let doc = parse(src);
        assert_eq!(marks(src, only_code(&doc)), ["````"]);
    }

    #[test]
    fn marks_in_quote_exclude_the_prefix() {
        let src = "> T\n> ===\n";
        assert_eq!(marks(src, &parse(src).blocks[1]), ["==="]);
        let src = "> a\n> ---\n";
        assert_eq!(marks(src, &parse(src).blocks[1]), ["---"]);
    }

    #[test]
    fn atx_closing_run() {
        let src = "## T ##\n";
        assert_eq!(marks(src, &parse(src).blocks[0]), ["##", "##"]);
        let src = "## T#\n";
        assert_eq!(marks(src, &parse(src).blocks[0]), ["##"]);
    }

    #[test]
    fn crlf_fence_and_setext() {
        let src = "```rust\r\nx\r\n```\r\n";
        assert_eq!(marks(src, &parse(src).blocks[0]), ["```rust", "```"]);
        let src = "Title\r\n===\r\n";
        assert_eq!(marks(src, &parse(src).blocks[0]), ["==="]);
    }

    #[test]
    fn strong_and_emphasis() {
        let src = "**a** *b* _c_ __d__";
        let doc = parse(src);
        let got: Vec<_> = doc
            .inlines
            .iter()
            .map(|i| (i.kind.clone(), imarks(src, i)))
            .collect();
        assert_eq!(
            got,
            [
                (InlineKind::Strong, vec!["**", "**"]),
                (InlineKind::Emphasis, vec!["*", "*"]),
                (InlineKind::Emphasis, vec!["_", "_"]),
                (InlineKind::Strong, vec!["__", "__"]),
            ]
        );
    }

    #[test]
    fn nested_emphasis() {
        let src = "***a***";
        let doc = parse(src);
        assert!(doc.inlines.iter().any(|i| i.kind == InlineKind::Strong));
        assert!(doc.inlines.iter().any(|i| i.kind == InlineKind::Emphasis));
        let mut stars: Vec<usize> = doc
            .inlines
            .iter()
            .flat_map(|i| i.marks.iter().flat_map(|m| m.clone()))
            .collect();
        for m in doc.inlines.iter().flat_map(|i| &i.marks) {
            assert!(m.end <= 7);
        }
        stars.sort();
        assert_eq!(stars, [0, 1, 2, 4, 5, 6]);
    }

    #[test]
    fn strikethrough() {
        let src = "~~a~~";
        let doc = parse(src);
        assert_eq!(doc.inlines.len(), 1);
        assert_eq!(doc.inlines[0].kind, InlineKind::Strikethrough);
        assert_eq!(imarks(src, &doc.inlines[0]), ["~~", "~~"]);
    }

    #[test]
    fn inline_code() {
        let src = "`` a ` b ``";
        let doc = parse(src);
        assert_eq!(doc.inlines.len(), 1);
        assert_eq!(doc.inlines[0].kind, InlineKind::Code);
        assert_eq!(imarks(src, &doc.inlines[0]), ["``", "``"]);
    }

    #[test]
    fn link() {
        let src = "[文字](https://e.x)";
        let doc = parse(src);
        assert_eq!(doc.inlines.len(), 1);
        let InlineKind::Link { url, text } = &doc.inlines[0].kind else {
            panic!("not a link");
        };
        assert_eq!(url, "https://e.x");
        assert_eq!(&src[text.clone()], "文字");
        assert_eq!(imarks(src, &doc.inlines[0]), ["[", "](https://e.x)"]);
    }

    #[test]
    fn reference_link() {
        let src = "[a][r]\n\n[r]: https://e.x\n";
        let doc = parse(src);
        assert_eq!(imarks(src, &doc.inlines[0]), ["[", "][r]"]);
    }

    #[test]
    fn autolink() {
        let src = "<https://e.x>";
        let doc = parse(src);
        assert_eq!(doc.inlines.len(), 1);
        assert!(matches!(doc.inlines[0].kind, InlineKind::Link { .. }));
        assert_eq!(imarks(src, &doc.inlines[0]), ["<", ">"]);
    }

    #[test]
    fn nothing_inside_code_blocks() {
        assert!(parse("```\n**a**\n```\n").inlines.is_empty());
    }

    #[test]
    fn sample_has_every_inline() {
        let doc = parse(SAMPLE);
        for want in [
            InlineKind::Emphasis,
            InlineKind::Strong,
            InlineKind::Strikethrough,
            InlineKind::Code,
        ] {
            assert!(doc.inlines.iter().any(|i| i.kind == want), "{want:?}");
        }
        let links = doc
            .inlines
            .iter()
            .filter(|i| matches!(i.kind, InlineKind::Link { .. }))
            .count();
        assert_eq!(links, 3);
    }

    #[test]
    fn every_prefix_parses() {
        for i in (0..=SAMPLE.len()).filter(|&i| SAMPLE.is_char_boundary(i)) {
            let src = &SAMPLE[..i];
            let doc = parse(src);
            let ok = |r: &Range<usize>| {
                r.start <= r.end
                    && r.end <= i
                    && src.is_char_boundary(r.start)
                    && src.is_char_boundary(r.end)
            };
            for b in &doc.blocks {
                assert!(ok(&b.range), "block {b:?} at {i}");
                assert!(b.marks.iter().all(ok), "block marks {b:?} at {i}");
            }
            for n in &doc.inlines {
                assert!(ok(&n.range), "inline {n:?} at {i}");
                assert!(n.marks.iter().all(ok), "inline marks {n:?} at {i}");
                if let InlineKind::Link { text, .. } = &n.kind {
                    assert!(ok(text), "link text {n:?} at {i}");
                }
            }
        }
    }
}
