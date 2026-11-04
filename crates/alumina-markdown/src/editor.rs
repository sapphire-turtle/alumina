//! [`MarkdownEditor`]: a `TextEdit` whose layouter lays the text out as Markdown, and which
//! takes the shortcut keys, Enter and Tab for the [`commands`](crate::commands).

use std::ops::Range;

use egui::cache::{ComputerMut, FrameCache};
use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::text_edit::{TextEditOutput, TextEditState};
use egui::{Context, Event, FocusDirection, Id, ImeEvent, Key, Modifiers, TextEdit, Ui};

use crate::commands::{Command, Edit, command, indent, newline};
use crate::layout::layout;
use crate::parse::{BlockKind, parse};
use crate::style::MarkdownStyle;

/// The byte offset of char `c` in `s`; past the end, `s.len()`.
pub(crate) fn char_to_byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b)
}

/// The char index of byte `b` in `s`; past the end, the char count.
pub(crate) fn byte_to_char(s: &str, b: usize) -> usize {
    s.char_indices().take_while(|(i, _)| *i < b).count()
}

/// The selection of the editor `id` in bytes of `text`; `None` if it has no cursor yet.
pub(crate) fn selection(ctx: &Context, id: Id, text: &str) -> Option<Range<usize>> {
    let range = TextEditState::load(ctx, id)?.cursor.char_range()?;
    let [start, end] = range.sorted_cursors();
    Some(char_to_byte(text, start.index.0)..char_to_byte(text, end.index.0))
}

/// Select `bytes` of `text` in the editor `id`.
#[allow(dead_code)] // For the toolbar (Task 11).
pub(crate) fn set_selection(ctx: &Context, id: Id, text: &str, bytes: Range<usize>) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(char_range(text, bytes)));
    state.store(ctx, id);
}

fn char_range(text: &str, bytes: Range<usize>) -> CCursorRange {
    CCursorRange::two(
        CCursor::new(byte_to_char(text, bytes.start)),
        CCursor::new(byte_to_char(text, bytes.end)),
    )
}

/// Apply `edit` to the editor `id` holding `text`, as one step for its undo.
fn apply(ctx: &Context, id: Id, text: &mut String, edit: &Edit) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    let mut undoer = state.undoer();
    if let Some(before) = state.cursor.char_range() {
        undoer.add_undo(&(before, text.clone()));
    }
    edit.apply(text);
    let after = char_range(text, edit.selection.clone());
    state.cursor.set_char_range(Some(after));
    undoer.add_undo(&(after, text.clone()));
    state.set_undoer(undoer);
    state.store(ctx, id);
}

/// The shortcuts for the inline commands. `Command-X` alone is cut.
const SHORTCUTS: [(Modifiers, Key, Command); 5] = [
    (Modifiers::COMMAND, Key::B, Command::Strong),
    (Modifiers::COMMAND, Key::I, Command::Emphasis),
    (Modifiers::COMMAND, Key::K, Command::Link),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::X,
        Command::Strikethrough,
    ),
    (Modifiers::COMMAND, Key::E, Command::Code),
];

/// Where the editor `id` keeps whether an IME composition is open.
fn composing_id(id: Id) -> Id {
    id.with("composing")
}

/// Take the keys the focused editor `id` handles out of this frame's input, and apply their
/// edits to `text` before the `TextEdit` sees the rest. Whether `text` changed.
fn take_keys(ui: &Ui, id: Id, text: &mut String) -> bool {
    let ctx = ui.ctx();
    if !ui.memory(|m| m.has_focus(id)) {
        ctx.data_mut(|d| d.remove::<bool>(composing_id(id)));
        return false;
    }
    let was_composing = ctx.data(|d| d.get_temp::<bool>(composing_id(id)).unwrap_or(false));
    let (composing, ime) = ui.input_mut(|i| take_ime_enters(&mut i.events, was_composing));
    ctx.data_mut(|d| d.insert_temp(composing_id(id), composing));

    let mut changed = false;
    for (modifiers, key, cmd) in SHORTCUTS {
        let Some(sel) = selection(ctx, id, text) else {
            return changed;
        };
        if ui.input_mut(|i| i.consume_key(modifiers, key))
            && let Some(edit) = command(text, sel, cmd)
        {
            apply(ctx, id, text, &edit);
            changed = true;
        }
    }
    let mut indented = false;
    for (modifiers, outdent) in [(Modifiers::NONE, false), (Modifiers::SHIFT, true)] {
        indented |= take_presses(ui, id, text, modifiers, Key::Tab, |text, sel| {
            indent(text, sel, outdent)
        });
    }
    if indented {
        changed = true;
        // The Tab was ours: egui's focus must not move on with it.
        ui.memory_mut(|m| m.move_focus(FocusDirection::None));
    }
    // Enter that commits a conversion is the IME's, not a new line's.
    if !ime && !composing {
        changed |= take_presses(ui, id, text, Modifiers::NONE, Key::Enter, |text, sel| {
            sel.is_empty().then(|| newline(text, sel.start)).flatten()
        });
    }
    changed
}

/// Each press of `key` with exactly `modifiers`, while `edit` has something to do: take
/// the press and apply the edit. Presses with nothing to do are left to egui.
fn take_presses(
    ui: &Ui,
    id: Id,
    text: &mut String,
    modifiers: Modifiers,
    key: Key,
    edit: impl Fn(&str, Range<usize>) -> Option<Edit>,
) -> bool {
    let is_press = |e: &Event| {
        matches!(e, Event::Key { key: k, pressed: true, modifiers: m, .. }
            if *k == key && m.matches_exact(modifiers))
    };
    let mut changed = false;
    while let Some(at) = ui.input(|i| i.events.iter().position(is_press)) {
        let Some(e) = selection(ui.ctx(), id, text).and_then(|sel| edit(text, sel)) else {
            break;
        };
        ui.input_mut(|i| i.events.remove(at));
        apply(ui.ctx(), id, text, &e);
        changed = true;
    }
    changed
}

/// Follow the IME through `events`, starting `composing` or not, and drop each Enter
/// pressed while a composition is open: it commits the conversion (some platforms deliver
/// it with the commit). Returns whether a composition is still open, and whether `events`
/// hold a preedit or a commit.
fn take_ime_enters(events: &mut Vec<Event>, mut composing: bool) -> (bool, bool) {
    let mut ime = false;
    events.retain(|e| match e {
        Event::Ime(ImeEvent::Preedit { text, .. }) => {
            ime = true;
            composing = !text.is_empty();
            true
        }
        Event::Ime(ImeEvent::Commit(_)) => {
            ime = true;
            composing = false;
            true
        }
        #[allow(deprecated)]
        Event::Ime(ImeEvent::Disabled) => {
            composing = false;
            true
        }
        Event::Key {
            key: Key::Enter,
            pressed: true,
            ..
        } => !composing,
        _ => true,
    });
    (composing, ime)
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
        let edited = !self.read_only && ui.is_enabled() && take_keys(ui, self.id, self.text);
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

        let mut text_edit = if self.read_only {
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
        if edited {
            text_edit.response.mark_changed();
        }
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
