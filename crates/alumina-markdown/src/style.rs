//! The look of the editor: fonts and colours.

use std::hash::{Hash, Hasher};

use egui::{Color32, FontFamily, FontId, Visuals};

/// Fonts and colours for [`layout`](crate::layout).
///
/// Plain data: change a field to restyle. [`from_visuals`](Self::from_visuals) gives the
/// defaults for the current theme.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownStyle {
    /// The font of text.
    pub body: FontId,
    /// The font of code.
    pub monospace: FontId,
    /// The font size of headings of level 1 to 6.
    pub heading_sizes: [f32; 6],
    pub text_color: Color32,
    /// The colour of strong text and headings. egui ships no bold face.
    pub strong_color: Color32,
    /// The colour of marks: `#`, `*`, backticks, fences, bullets.
    pub mark_color: Color32,
    pub link_color: Color32,
    pub quote_color: Color32,
    pub code_background: Color32,
}

impl MarkdownStyle {
    /// The defaults, following the dark or light theme of `visuals`.
    pub fn from_visuals(visuals: &Visuals) -> Self {
        Self {
            body: FontId::new(14.0, FontFamily::Proportional),
            monospace: FontId::new(13.0, FontFamily::Monospace),
            heading_sizes: [24.0, 20.0, 18.0, 16.0, 14.0, 14.0],
            text_color: visuals.text_color(),
            strong_color: visuals.strong_text_color(),
            mark_color: visuals.weak_text_color(),
            link_color: visuals.hyperlink_color,
            quote_color: visuals.weak_text_color(),
            code_background: visuals.code_bg_color,
        }
    }
}

impl Hash for MarkdownStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for font in [&self.body, &self.monospace] {
            font.size.to_bits().hash(state);
            font.family.hash(state);
        }
        for size in self.heading_sizes {
            size.to_bits().hash(state);
        }
        for color in [
            self.text_color,
            self.strong_color,
            self.mark_color,
            self.link_color,
            self.quote_color,
            self.code_background,
        ] {
            color.hash(state);
        }
    }
}
