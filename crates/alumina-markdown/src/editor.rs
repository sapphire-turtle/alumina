//! [`MarkdownEditor`]: a `TextEdit` whose layouter lays the text out as Markdown.

use egui::cache::{ComputerMut, FrameCache};
use egui::text::{CCursor, LayoutJob};
use egui::text_edit::TextEditOutput;
use egui::{Id, TextEdit, Ui};

use crate::layout::layout;
use crate::parse::{BlockKind, parse};
use crate::style::MarkdownStyle;

/// The byte offset of char `c` in `s`; past the end, `s.len()`.
#[allow(dead_code)] // Used by the commands (Task 7 on).
pub(crate) fn char_to_byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b)
}

/// The char index of byte `b` in `s`; past the end, the char count.
pub(crate) fn byte_to_char(s: &str, b: usize) -> usize {
    s.char_indices().take_while(|(i, _)| *i < b).count()
}

/// Parses and lays out, the work the cache saves.
#[derive(Default)]
struct JobComputer;

/// One parse: the job, and the byte offsets of the thematic breaks to draw.
#[derive(Clone)]
struct Rendered {
    job: LayoutJob,
    rules: Vec<usize>,
}

impl<'a> ComputerMut<(&'a str, &'a MarkdownStyle, u32), Rendered> for JobComputer {
    fn compute(&mut self, (text, style, width): (&'a str, &'a MarkdownStyle, u32)) -> Rendered {
        let doc = parse(text);
        let rules = doc
            .blocks
            .iter()
            .filter(|b| b.kind == BlockKind::ThematicBreak)
            .map(|b| b.range.start)
            .collect();
        Rendered {
            job: layout(text, &doc, style, f32::from_bits(width)),
            rules,
        }
    }
}

type JobCache = FrameCache<Rendered, JobComputer>;

/// A Markdown editor, or a read-only view with the same look.
pub struct MarkdownEditor<'t> {
    text: &'t mut String,
    id: Id,
    read_only: bool,
    style: Option<MarkdownStyle>,
    desired_rows: usize,
}

impl<'t> MarkdownEditor<'t> {
    pub fn new(text: &'t mut String) -> Self {
        Self {
            text,
            id: Id::unique("alumina-markdown"),
            read_only: false,
            style: None,
            desired_rows: 8,
        }
    }

    /// The id of the `TextEdit`, to find its state before the editor shows.
    ///
    /// The default is shared by all editors: two in one window need an id each.
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Selectable and copyable, but not editable.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Defaults to [`MarkdownStyle::from_visuals`] of the `ui`.
    pub fn style(mut self, style: MarkdownStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// The height in rows of text; defaults to 8.
    pub fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    pub fn show(self, ui: &mut Ui) -> MarkdownOutput {
        let style = self
            .style
            .unwrap_or_else(|| MarkdownStyle::from_visuals(ui.visuals()));
        // Set by each call of the layouter, so the last one (after any edit) wins.
        let mut rules = Vec::new();
        let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let rendered = ui.memory_mut(|m| {
                m.caches
                    .cache::<JobCache>()
                    .get((buf.as_str(), &style, wrap_width.to_bits()))
                    .clone()
            });
            rules = rendered.rules;
            ui.fonts_mut(|f| f.layout_job(rendered.job))
        };

        let text_edit = if self.read_only {
            let mut view: &str = self.text.as_str();
            TextEdit::multiline(&mut view)
                .id(self.id)
                .desired_rows(self.desired_rows)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter)
                .show(ui)
        } else {
            TextEdit::multiline(self.text)
                .id(self.id)
                .desired_rows(self.desired_rows)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter)
                .show(ui)
        };

        // Screen readers and tests find the field by this name.
        ui.ctx().accesskit_node_builder(self.id, |node| {
            node.set_label("Markdown");
        });
        paint_rules(ui, &text_edit, self.text.as_str(), &rules);
        MarkdownOutput {
            // A read-only view reports the keys it ignored as changes.
            changed: !self.read_only && text_edit.response.changed(),
            link_clicked: None,
            text_edit,
        }
    }
}

/// Draws a line through the row of each thematic break, given by byte offset.
fn paint_rules(ui: &Ui, out: &TextEditOutput, source: &str, rules: &[usize]) {
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    let clip = out.text_clip_rect;
    let painter = ui.painter_at(clip);
    for &start in rules {
        let at = CCursor::new(byte_to_char(source, start));
        let y = out.galley_pos.y + out.galley.pos_from_cursor(at).center().y;
        painter.hline(clip.x_range(), y, stroke);
    }
}

/// What [`MarkdownEditor::show`] returns.
pub struct MarkdownOutput {
    /// The text was edited this frame.
    pub changed: bool,
    /// A link was clicked, with its destination.
    pub link_clicked: Option<String>,
    pub text_edit: TextEditOutput,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_chars_and_bytes() {
        let s = "a日本🎌b";
        assert_eq!(char_to_byte(s, 0), 0);
        assert_eq!(char_to_byte(s, 2), 4);
        assert_eq!(char_to_byte(s, 4), 11);
        assert_eq!(char_to_byte(s, 5), s.len());
        assert_eq!(byte_to_char(s, 4), 2);
        assert_eq!(byte_to_char(s, 11), 4);
        assert_eq!(byte_to_char(s, s.len()), 5);
    }

    #[test]
    fn the_cached_value_carries_the_rules() {
        let style = MarkdownStyle::from_visuals(&egui::Visuals::dark());
        let r = JobComputer.compute(("a\n\n---\n", &style, 100.0f32.to_bits()));
        assert_eq!(r.rules, vec![3]);
        assert_eq!(r.job.text, "a\n\n---\n");
    }

    #[test]
    fn out_of_range_clamps_to_the_end() {
        assert_eq!(char_to_byte("日本", 9), 6);
        assert_eq!(byte_to_char("日本", 99), 2);
        assert_eq!(char_to_byte("", 3), 0);
    }
}
