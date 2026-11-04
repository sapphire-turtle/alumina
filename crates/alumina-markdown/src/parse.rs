//! Source to [`Doc`]: the blocks of a Markdown text and where their marks are.
//!
//! Every range is a byte range of the source, on char boundaries. Marks (the `#`s, the
//! bullet, the `>`s, the fences) are never dropped from the text, only located.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// A parsed source: its blocks and inlines, in source order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Doc {
    /// Nested blocks come after their parent.
    pub blocks: Vec<Block>,
    /// Not read yet.
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
                _ => {}
            },
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
}
