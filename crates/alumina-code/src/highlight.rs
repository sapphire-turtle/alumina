//! Syntax highlighting the app supplies: the [`Highlighter`] trait, the [`TokenKind`]s it
//! speaks in, and the [`CodeTheme`] that colours them. alumina knows no language itself.

use std::ops::Range;

use egui::text::{ByteIndex, LayoutJob, LayoutSection, TextFormat};
use egui::{Color32, FontId, Visuals};

/// What a highlighter marks a range of code as.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    /// A comment.
    Comment,
    /// A keyword.
    Keyword,
    /// A type name.
    Type,
    /// A function name.
    Function,
    /// A string literal.
    String,
    /// A number literal.
    Number,
    /// A constant, such as `true` or `null`.
    Constant,
    /// A key, such as a JSON or TOML key.
    Key,
    /// Punctuation and operators.
    Punctuation,
    /// An attribute or annotation.
    Attribute,
}

impl TokenKind {
    const ALL: [TokenKind; 10] = [
        TokenKind::Comment,
        TokenKind::Keyword,
        TokenKind::Type,
        TokenKind::Function,
        TokenKind::String,
        TokenKind::Number,
        TokenKind::Constant,
        TokenKind::Key,
        TokenKind::Punctuation,
        TokenKind::Attribute,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|k| *k == self).unwrap_or(0)
    }
}

/// Finds the tokens of a piece of code. The app implements it with whatever it likes (a
/// regex, `syntect`, tree-sitter).
pub trait Highlighter {
    /// The tokens of `code` written in `language` (a name such as `"rust"`, as the app
    /// uses it), as byte ranges of `code`; `None` if the language is not known, which
    /// leaves the code plain.
    ///
    /// The ranges are not trusted: those past the end, empty, reversed or off a char
    /// boundary are dropped, and where they overlap the later one wins.
    fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>>;
}

/// The colours of the [`TokenKind`]s.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodeTheme {
    colors: [Color32; 10],
}

impl CodeTheme {
    /// A theme that reads on `visuals`: dark or light by `visuals.dark_mode`.
    pub fn from_visuals(visuals: &Visuals) -> Self {
        let dark = visuals.dark_mode;
        let pick = |d: u32, l: u32| {
            let rgb = if dark { d } else { l };
            Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
        };
        let colors = TokenKind::ALL.map(|kind| match kind {
            TokenKind::Comment => visuals.weak_text_color(),
            TokenKind::Punctuation => visuals.text_color(),
            TokenKind::Keyword => pick(0xC678DD, 0xA626A4),
            TokenKind::Type => pick(0xE5C07B, 0xC18401),
            TokenKind::Function => pick(0x61AFEF, 0x4078F2),
            TokenKind::String => pick(0x98C379, 0x50A14F),
            TokenKind::Number | TokenKind::Constant => pick(0xD19A66, 0x986801),
            TokenKind::Key => pick(0xE06C75, 0xE45649),
            TokenKind::Attribute => pick(0x56B6C2, 0x0184BC),
        });
        Self { colors }
    }

    /// The colour of `kind`.
    pub fn color(&self, kind: TokenKind) -> Color32 {
        self.colors[kind.index()]
    }

    /// Set the colour of `kind`.
    pub fn set_color(&mut self, kind: TokenKind, color: Color32) {
        self.colors[kind.index()] = color;
    }
}

impl Default for CodeTheme {
    /// [`CodeTheme::from_visuals`] of the default (dark) visuals.
    fn default() -> Self {
        Self::from_visuals(&Visuals::default())
    }
}

/// The layout of `code` with `tokens` coloured by `theme` and the rest in `color`, wrapped
/// at `wrap_width`. One section per run of equal colour.
///
/// The ranges are sanitised: clamped to `code`, and dropped when empty, reversed or not on
/// char boundaries; they are painted in order, so the later wins where they overlap.
pub fn highlighted_job(
    code: &str,
    tokens: &[(Range<usize>, TokenKind)],
    font: FontId,
    color: Color32,
    theme: &CodeTheme,
    wrap_width: f32,
) -> LayoutJob {
    let mut colors = vec![color; code.len()];
    for (range, kind) in tokens {
        let start = range.start.min(code.len());
        let end = range.end.min(code.len());
        if start >= end || !code.is_char_boundary(start) || !code.is_char_boundary(end) {
            continue;
        }
        colors[start..end].fill(theme.color(*kind));
    }

    let mut job = LayoutJob {
        text: code.to_owned(),
        ..LayoutJob::default()
    };
    job.wrap.max_width = wrap_width;
    if code.is_empty() {
        // As `LayoutJob::simple`: an empty text still has a section, for its row height.
        job.sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: ByteIndex(0)..ByteIndex(0),
            format: TextFormat::simple(font.clone(), color),
        });
    }
    let mut start = 0;
    while start < code.len() {
        let run = colors[start];
        let end = colors[start..]
            .iter()
            .position(|c| *c != run)
            .map_or(code.len(), |n| start + n);
        job.sections.push(LayoutSection {
            leading_space: 0.0,
            byte_range: ByteIndex(start)..ByteIndex(end),
            format: TextFormat::simple(font.clone(), run),
        });
        start = end;
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(code: &str, tokens: &[(Range<usize>, TokenKind)]) -> LayoutJob {
        highlighted_job(
            code,
            tokens,
            FontId::monospace(12.0),
            Color32::WHITE,
            &CodeTheme::default(),
            100.0,
        )
    }

    #[test]
    fn empty_code_keeps_one_section() {
        let j = job("", &[]);
        assert_eq!(j.sections.len(), 1);
        assert_eq!(j.sections[0].format.color, Color32::WHITE);
    }

    #[test]
    fn a_token_is_a_section() {
        let theme = CodeTheme::default();
        let j = job("fn a", &[(0..2, TokenKind::Keyword)]);
        assert_eq!(j.sections.len(), 2);
        assert_eq!(j.sections[0].format.color, theme.color(TokenKind::Keyword));
        assert_eq!(j.sections[1].format.color, Color32::WHITE);
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn garbage_ranges_do_not_panic() {
        let tokens = [
            (2..99, TokenKind::String),
            (3..1, TokenKind::Number),
            (1..2, TokenKind::Key),
        ];
        let j = job("日本", &tokens);
        assert_eq!(j.text, "日本");
        for s in &j.sections {
            assert!(j.text.is_char_boundary(s.byte_range.start.0));
            assert!(j.text.is_char_boundary(s.byte_range.end.0));
        }
    }

    #[test]
    fn the_later_range_wins() {
        let theme = CodeTheme::default();
        let j = job(
            "abcd",
            &[(0..4, TokenKind::String), (1..2, TokenKind::Number)],
        );
        let at = |b: usize| {
            j.sections
                .iter()
                .find(|s| s.byte_range.contains(&ByteIndex(b)))
                .unwrap()
                .format
                .color
        };
        assert_eq!(at(1), theme.color(TokenKind::Number));
        assert_eq!(at(0), theme.color(TokenKind::String));
    }

    #[test]
    fn dark_and_light_differ() {
        assert_ne!(
            CodeTheme::from_visuals(&Visuals::dark()),
            CodeTheme::from_visuals(&Visuals::light())
        );
    }

    #[test]
    fn set_color_round_trips() {
        let mut theme = CodeTheme::default();
        theme.set_color(TokenKind::Type, Color32::RED);
        assert_eq!(theme.color(TokenKind::Type), Color32::RED);
    }
}
