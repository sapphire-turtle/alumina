//! [`Doc`] and [`MarkdownStyle`] to a [`LayoutJob`].

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Stroke};

use crate::parse::{BlockKind, Doc, InlineKind};
use crate::style::MarkdownStyle;

/// The format of one byte, before it is made a [`TextFormat`].
#[derive(Clone, Copy, PartialEq)]
struct Fmt {
    size: f32,
    mono: bool,
    color: Color32,
    background: Color32,
    italics: bool,
    strike: bool,
    underline: bool,
}

/// Lays `source` out as one job whose text is `source`, a section per run of equal format.
///
/// The formats come from the blocks (outer to inner), then the inlines, then the marks,
/// which only change the colour. Every byte is in exactly one section.
pub fn layout(source: &str, doc: &Doc, style: &MarkdownStyle, wrap_width: f32) -> LayoutJob {
    let mut fmts = vec![
        Fmt {
            size: style.body.size,
            mono: false,
            color: style.text_color,
            background: Color32::TRANSPARENT,
            italics: false,
            strike: false,
            underline: false,
        };
        source.len()
    ];
    let len = source.len();
    let mut paint = |range: &std::ops::Range<usize>, f: &dyn Fn(&mut Fmt)| {
        let range = range.start.min(len)..range.end.min(len);
        if range.start < range.end {
            fmts[range].iter_mut().for_each(f);
        }
    };

    for block in &doc.blocks {
        match &block.kind {
            BlockKind::Heading { level } => {
                let size = style.heading_sizes[usize::from((*level).clamp(1, 6)) - 1];
                paint(&block.range, &|f| {
                    f.size = size;
                    f.color = style.strong_color;
                });
            }
            BlockKind::Quote { .. } => paint(&block.range, &|f| f.color = style.quote_color),
            BlockKind::CodeBlock { .. } => paint(&block.range, &|f| {
                f.mono = true;
                f.size = style.monospace.size;
                f.background = style.code_background;
            }),
            _ => {}
        }
    }
    for inline in &doc.inlines {
        match &inline.kind {
            InlineKind::Emphasis => paint(&inline.range, &|f| f.italics = true),
            InlineKind::Strong => paint(&inline.range, &|f| f.color = style.strong_color),
            InlineKind::Strikethrough => paint(&inline.range, &|f| f.strike = true),
            InlineKind::Code => paint(&inline.range, &|f| {
                f.mono = true;
                f.size = style.monospace.size;
                f.background = style.code_background;
            }),
            InlineKind::Link { .. } => paint(&inline.range, &|f| {
                f.color = style.link_color;
                f.underline = true;
            }),
        }
    }
    let marks = doc
        .blocks
        .iter()
        .flat_map(|b| &b.marks)
        .chain(doc.inlines.iter().flat_map(|i| &i.marks));
    for mark in marks {
        paint(mark, &|f| f.color = style.mark_color);
    }

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.text.reserve(len);
    let mut start = 0;
    while start < len {
        let fmt = fmts[start];
        let mut end = start + 1;
        while end < len && fmts[end] == fmt {
            end += 1;
        }
        // Formats change only where ranges start or end, which are char boundaries.
        debug_assert!(source.is_char_boundary(end));
        job.append(&source[start..end], 0.0, text_format(&fmt, style));
        start = end;
    }
    job
}

fn text_format(fmt: &Fmt, style: &MarkdownStyle) -> TextFormat {
    let family = if fmt.mono {
        style.monospace.family.clone()
    } else {
        style.body.family.clone()
    };
    TextFormat {
        font_id: FontId::new(fmt.size, family),
        color: fmt.color,
        background: fmt.background,
        italics: fmt.italics,
        underline: if fmt.underline {
            Stroke::new(1.0, fmt.color)
        } else {
            Stroke::NONE
        },
        strikethrough: if fmt.strike {
            Stroke::new(1.0, fmt.color)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use egui::epaint::text::ByteRangeExt as _;

    use super::*;
    use crate::parse::parse;

    const SAMPLE: &str = include_str!("../tests/sample.md");

    fn style() -> MarkdownStyle {
        MarkdownStyle::from_visuals(&egui::Visuals::dark())
    }

    fn job(source: &str) -> LayoutJob {
        layout(source, &parse(source), &style(), 300.0)
    }

    fn at(job: &LayoutJob, byte: usize) -> &TextFormat {
        &job.sections
            .iter()
            .find(|s| s.byte_range.as_usize().contains(&byte))
            .expect("byte in a section")
            .format
    }

    #[test]
    fn job_text_is_the_source() {
        let job = job(SAMPLE);
        assert_eq!(job.text, SAMPLE);
        let mut next = 0;
        for s in &job.sections {
            let r = s.byte_range.as_usize();
            assert_eq!(r.start, next);
            next = r.end;
        }
        assert_eq!(next, SAMPLE.len());
    }

    #[test]
    fn heading_size() {
        let job = job("## T\n");
        assert_eq!(at(&job, 3).font_id.size, 20.0);
    }

    #[test]
    fn marks_are_faint() {
        let job = job("**a**");
        let s = style();
        assert_eq!(at(&job, 0).color, s.mark_color);
        assert_eq!(at(&job, 3).color, s.mark_color);
        assert_eq!(at(&job, 2).color, s.strong_color);
        let job = self::job("*a*");
        assert_eq!(at(&job, 1).color, s.text_color);
    }

    #[test]
    fn strong_and_emphasis_and_strike() {
        let s = style();
        let j = job("**a** *b* ~~c~~");
        assert_eq!(at(&j, 2).color, s.strong_color);
        assert!(at(&j, 7).italics);
        assert!(!at(&j, 2).italics);
        assert_ne!(at(&j, 13).strikethrough, Stroke::NONE);
        assert_eq!(at(&j, 2).strikethrough, Stroke::NONE);
    }

    #[test]
    fn code() {
        let s = style();
        let j = job("`a`");
        assert_eq!(at(&j, 1).font_id, s.monospace);
        assert_eq!(at(&j, 1).background, s.code_background);
        let j = job("```\n**a**\n```\n");
        assert_eq!(at(&j, 5).font_id, s.monospace);
        assert_eq!(at(&j, 5).background, s.code_background);
        assert_eq!(at(&j, 5).color, s.text_color);
        let j = job("> ```\n> x\n> ```\n");
        assert_eq!(at(&j, 8).font_id, s.monospace);
    }

    #[test]
    fn link() {
        let s = style();
        let j = job("[ab](http://x)");
        assert_eq!(at(&j, 1).color, s.link_color);
        assert_ne!(at(&j, 1).underline, Stroke::NONE);
    }

    #[test]
    fn wrap_width() {
        let j = layout("a", &parse("a"), &style(), 123.0);
        assert_eq!(j.wrap.max_width, 123.0);
    }

    #[test]
    fn multibyte_and_empty() {
        let j = job("日本語🎌 **語**");
        assert_eq!(j.text, "日本語🎌 **語**");
        assert_eq!(job("").sections.len(), 0);
    }
}
