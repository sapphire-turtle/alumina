//! [`MarkdownEditor`]: a `TextEdit` whose layouter lays the text out as Markdown, and which
//! takes the shortcut keys, Enter and Tab for the [`commands`](crate::commands).

use std::ops::Range;

use egui::cache::{ComputerMut, FrameCache};
use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::text_edit::{TextEditOutput, TextEditState};
use egui::{
    Context, CursorIcon, Event, FocusDirection, Id, ImeEvent, Key, Modifiers, TextEdit, Ui,
};

use crate::commands::{Command, Edit, command, indent, newline};
use crate::layout::layout;
use crate::parse::{BlockKind, InlineKind, parse};
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
pub(crate) fn apply(ctx: &Context, id: Id, text: &mut String, edit: &Edit) {
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

/// What a key the editor takes does.
#[derive(Clone, Copy)]
enum Action {
    Command(Command),
    /// Tab, or Shift-Tab with `true`.
    Indent(bool),
    Newline,
}

/// The keys the editor takes, with exactly these modifiers: an extra Alt is AltGr typing a
/// char on Windows (Ctrl+Alt), an extra Shift is someone else's shortcut. `Command-X` alone
/// is cut.
const KEYS: [(Modifiers, Key, Action); 8] = [
    (Modifiers::COMMAND, Key::B, Action::Command(Command::Strong)),
    (
        Modifiers::COMMAND,
        Key::I,
        Action::Command(Command::Emphasis),
    ),
    (Modifiers::COMMAND, Key::K, Action::Command(Command::Link)),
    (
        Modifiers::COMMAND.plus(Modifiers::SHIFT),
        Key::X,
        Action::Command(Command::Strikethrough),
    ),
    (Modifiers::COMMAND, Key::E, Action::Command(Command::Code)),
    (Modifiers::NONE, Key::Tab, Action::Indent(false)),
    (Modifiers::SHIFT, Key::Tab, Action::Indent(true)),
    (Modifiers::NONE, Key::Enter, Action::Newline),
];

/// The shortcut of `cmd` in [`KEYS`], if it has one.
pub(crate) fn shortcut(cmd: Command) -> Option<egui::KeyboardShortcut> {
    KEYS.iter().find_map(|&(m, k, a)| match a {
        Action::Command(c) if c == cmd => Some(egui::KeyboardShortcut::new(m, k)),
        _ => None,
    })
}

/// The action of `event`, if it is a press of one of [`KEYS`].
fn action(event: &Event) -> Option<Action> {
    let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    KEYS.iter()
        .find(|(m, k, _)| k == key && modifiers.matches_exact(*m))
        .map(|&(_, _, action)| action)
}

/// Where the editor `id` keeps whether an IME composition is open.
fn composing_id(id: Id) -> Id {
    id.with("composing")
}

/// Forget an open IME composition: the editor is not taking keys this frame, so it cannot
/// follow one, and a stale flag would swallow every later Enter.
fn forget_composing(ctx: &Context, id: Id) {
    ctx.data_mut(|d| d.remove::<bool>(composing_id(id)));
}

/// Take the keys the focused, editable editor `id` handles out of this frame's input, and
/// apply their edits to `text` before the `TextEdit` sees the rest. Whether `text` changed.
fn take_keys(ui: &Ui, id: Id, text: &mut String) -> bool {
    let ctx = ui.ctx();
    let was_composing = ctx.data(|d| d.get_temp::<bool>(composing_id(id)).unwrap_or(false));
    let (composing, ime) = ui.input_mut(|i| take_ime_enters(&mut i.events, was_composing));
    ctx.data_mut(|d| d.insert_temp(composing_id(id), composing));

    // The presses to act on, in order; most frames have none.
    let presses: Vec<(usize, Action)> = ui.input(|i| {
        i.events
            .iter()
            .enumerate()
            .filter_map(|(at, e)| action(e).map(|a| (at, a)))
            .collect()
    });
    if presses.is_empty() {
        return false;
    }
    // With no cursor yet (focus just gained), act at the end of the text.
    let mut sel = selection(ctx, id, text).unwrap_or(text.len()..text.len());
    let mut taken = 0;
    let mut indented = false;
    let mut changed = false;
    for (at, action) in presses {
        let edit = match action {
            Action::Command(cmd) => command(text, sel.clone(), cmd),
            Action::Indent(outdent) => indent(text, sel.clone(), outdent),
            // Enter that commits a conversion is the IME's, not a new line's.
            Action::Newline if ime || composing || !sel.is_empty() => None,
            Action::Newline => newline(text, sel.start),
        };
        // Nothing to do: the key is egui's.
        let Some(edit) = edit else { continue };
        ui.input_mut(|i| i.events.remove(at - taken));
        taken += 1;
        indented |= matches!(action, Action::Indent(_));
        apply(ctx, id, text, &edit);
        sel = edit.selection;
        changed = true;
    }
    if indented {
        // The Tab was ours: egui's focus must not move on with it.
        ui.memory_mut(|m| m.move_focus(FocusDirection::None));
    }
    changed
}

/// Follow the IME through `events`, starting `composing` or not, and drop each Enter
/// pressed while a composition is open: it commits the conversion. This holds outside lists
/// too: native egui-winit never delivers that Enter, but on the web a composing Enter
/// arrives as `Key::Enter`, which egui would turn into a newline (Ruling R7).
///
/// Returns whether a composition is still open, and whether `events` hold a preedit or a
/// commit.
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
        // egui-winit discards winit's `Ime::Enabled`/`Disabled`; this is for other
        // integrations.
        #[allow(deprecated)]
        Event::Ime(ImeEvent::Disabled) => {
            composing = false;
            true
        }
        // Plain typing: whatever composition there was is over.
        Event::Text(_) => {
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

/// One parse: the job, the byte offsets of the thematic breaks to draw, and the links
/// (byte range and destination) to click.
#[derive(Clone)]
struct Rendered {
    job: LayoutJob,
    rules: Vec<usize>,
    links: Vec<(Range<usize>, String)>,
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
        let links = doc
            .inlines
            .iter()
            .filter_map(|i| match &i.kind {
                InlineKind::Link { url, .. } => Some((i.range.clone(), url.clone())),
                _ => None,
            })
            .collect();
        Rendered {
            links,
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
        let takes_keys = !self.read_only && ui.is_enabled() && ui.memory(|m| m.has_focus(self.id));
        let edited = if takes_keys {
            take_keys(ui, self.id, self.text)
        } else {
            forget_composing(ui.ctx(), self.id);
            false
        };
        let style = self
            .style
            .unwrap_or_else(|| MarkdownStyle::from_visuals(ui.visuals()));
        // Set by each call of the layouter, so the last one (after any edit) wins.
        let mut rules = Vec::new();
        let mut links = Vec::new();
        let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let rendered = ui.memory_mut(|m| {
                m.caches
                    .cache::<JobCache>()
                    .get((buf.as_str(), &style, wrap_width.to_bits()))
                    .clone()
            });
            rules = rendered.rules;
            links = rendered.links;
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
        let link_clicked =
            link_at_pointer(ui, &text_edit, self.text.as_str(), &links, self.read_only);
        MarkdownOutput {
            // A read-only view reports the keys it ignored as changes.
            changed: !self.read_only && text_edit.response.changed(),
            link_clicked,
            text_edit,
        }
    }
}

/// The destination of the link under the pointer where a click would report it (always when
/// `read_only`, else with the command key held): sets the pointing hand, and returns the link
/// if this frame's click is on it.
fn link_at_pointer(
    ui: &Ui,
    out: &TextEditOutput,
    source: &str,
    links: &[(Range<usize>, String)],
    read_only: bool,
) -> Option<String> {
    if links.is_empty() || !(read_only || ui.input(|i| i.modifiers.command)) {
        return None;
    }
    let response = &out.response;
    let pointer = response.hover_pos()?;
    let galley = &out.galley;
    let rel = pointer - out.galley_pos;
    // The nearest cursor exists even in the empty space right of a row: the pointer must be
    // over the text.
    if !galley.rows.iter().any(|r| r.rect().contains(rel.to_pos2())) {
        return None;
    }
    let cursor = galley.cursor_from_pos(rel);
    // The cursor is the nearest edge: left of it, the pointer is on the char before.
    let mut at = cursor.index.0;
    if at > 0 && rel.x < galley.pos_from_cursor(cursor).min.x {
        at -= 1;
    }
    let byte = char_to_byte(source, at);
    let url = links
        .iter()
        .find(|(range, _)| range.contains(&byte))
        .map(|(_, url)| url)?;
    ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
    // `clicked` is the primary button.
    response.clicked().then(|| url.clone())
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
    fn the_cached_value_carries_the_links() {
        let style = MarkdownStyle::from_visuals(&egui::Visuals::dark());
        let r = JobComputer.compute(("a [x](u) b", &style, 100.0f32.to_bits()));
        assert_eq!(r.links, vec![(2..8, "u".to_string())]);
    }

    #[test]
    fn out_of_range_clamps_to_the_end() {
        assert_eq!(char_to_byte("日本", 9), 6);
        assert_eq!(byte_to_char("日本", 99), 2);
        assert_eq!(char_to_byte("", 3), 0);
    }
}
