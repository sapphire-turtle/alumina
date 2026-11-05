//! [`MarkdownEditor`]: a `TextEdit` whose layouter lays the text out as Markdown, and which
//! takes the shortcut keys, Enter and Tab for the [`commands`](crate::commands).

use std::hash::{Hash as _, Hasher as _};
use std::ops::Range;

use alumina_code::Highlighter;
use egui::cache::{ComputerMut, FrameCache};
use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::text_edit::{TextEditOutput, TextEditState};
use egui::{
    Context, CursorIcon, Event, FocusDirection, Id, ImeEvent, Key, Modifiers, TextEdit, Ui,
};

use crate::commands::{Command, Edit, command, indent, newline};
use crate::layout::layout_highlighted;
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

/// Where the editor `id` finds that its text was edited this frame by something other than
/// its `TextEdit` (the toolbar), so that it scrolls to the cursor.
fn edited_id(id: Id) -> Id {
    id.with("edited")
}

/// Tell the editor `id` that its text and cursor were changed for it: it scrolls to the
/// cursor when it next shows.
pub(crate) fn mark_edited(ctx: &Context, id: Id) {
    ctx.data_mut(|d| d.insert_temp(edited_id(id), true));
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

/// Whether `event`, left for egui, edits the text or moves the cursor. A pressed key counts
/// only if the `TextEdit` acts on it: the arrows, Home, End, paging, deleting, Enter, Tab,
/// and the shortcuts. A char key is not an edit: its `Text` event is (egui-winit sends the
/// key before the text, the web the text before the key).
///
/// Copied from alumina-code's editor, to avoid new public API there.
fn egui_edits(event: &Event) -> bool {
    match event {
        Event::Text(_) | Event::Paste(_) | Event::Cut | Event::Ime(_) => true,
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            modifiers.ctrl
                || modifiers.command
                || modifiers.mac_cmd
                || matches!(
                    key,
                    Key::ArrowUp
                        | Key::ArrowDown
                        | Key::ArrowLeft
                        | Key::ArrowRight
                        | Key::Home
                        | Key::End
                        | Key::PageUp
                        | Key::PageDown
                        | Key::Backspace
                        | Key::Delete
                        | Key::Enter
                        | Key::Tab
                )
        }
        _ => false,
    }
}

/// Take the keys the focused, editable editor `id` handles out of this frame's input, and
/// apply their edits to `text` before the `TextEdit` sees the rest. Whether `text` changed.
///
/// The edits come before egui's, so the editor takes presses only up to the first event of
/// the frame that it leaves for egui and that edits the text or moves the cursor (a `Text`,
/// `Paste`, `Cut`, `Ime` event, or a pressed key it does not take that egui acts on,
/// including a taken key whose command is `None`, such as Enter outside a list). From that
/// event on it takes nothing more this frame: the later presses reach egui as they are, so
/// that the text comes out in the order typed. (This is the rule of alumina-code's editor.)
fn take_keys(ui: &Ui, id: Id, text: &mut String) -> bool {
    let ctx = ui.ctx();
    let was_composing = ctx.data(|d| d.get_temp::<bool>(composing_id(id)).unwrap_or(false));
    let (composing, ime) = ui.input_mut(|i| take_ime_enters(&mut i.events, was_composing));
    ctx.data_mut(|d| d.insert_temp(composing_id(id), composing));

    // The presses to act on, in order, up to the first event egui will handle that edits the
    // text or moves the cursor; most frames have none.
    let presses: Vec<(usize, Action)> = ui.input(|i| {
        let mut presses = Vec::new();
        for (at, e) in i.events.iter().enumerate() {
            match action(e) {
                Some(a) => presses.push((at, a)),
                None if egui_edits(e) => break,
                None => {}
            }
        }
        presses
    });
    if presses.is_empty() {
        return false;
    }
    // With no cursor yet (focus just gained), act at the end of the text.
    let mut sel = selection(ctx, id, text).unwrap_or(text.len()..text.len());
    let mut indented = false;
    let mut changed = false;
    // Each press that does not end the loop is taken, so `taken` is its index.
    for (taken, (at, action)) in presses.into_iter().enumerate() {
        let edit = match action {
            Action::Command(cmd) => command(text, sel.clone(), cmd),
            Action::Indent(outdent) => indent(text, sel.clone(), outdent),
            // Enter that commits a conversion is the IME's, not a new line's.
            Action::Newline if ime || composing || !sel.is_empty() => None,
            Action::Newline => newline(text, sel.start),
        };
        // Nothing to do: the key is egui's, which edits at the cursor: what follows must
        // come after it.
        let Some(edit) = edit else { break };
        ui.input_mut(|i| i.events.remove(at - taken));
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
/// too: eframe's native and web integrations drop such an Enter themselves, but an
/// integration that delivers an Enter key with the commit would have egui turn it into a
/// newline (Ruling R7).
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

/// One parse: the job, the char indices of the thematic breaks to draw, in order, and the
/// links (char range and destination) to click.
#[derive(Clone)]
struct Rendered {
    job: LayoutJob,
    rules: Vec<usize>,
    links: Vec<(Range<usize>, String)>,
}

impl<'a> ComputerMut<(&'a str, &'a MarkdownStyle, u32), Rendered> for JobComputer {
    fn compute(&mut self, (text, style, width): (&'a str, &'a MarkdownStyle, u32)) -> Rendered {
        render(text, style, f32::from_bits(width), None)
    }
}

/// Parses and lays out `text`, with its code coloured by `highlighter` if there is one.
fn render(
    text: &str,
    style: &MarkdownStyle,
    width: f32,
    highlighter: Option<&dyn Highlighter>,
) -> Rendered {
    {
        let doc = parse(text);
        let starts: Vec<usize> = doc
            .blocks
            .iter()
            .filter(|b| b.kind == BlockKind::ThematicBreak)
            .map(|b| b.range.start)
            .collect();
        let rules = byte_starts_to_chars(text, &starts);
        let links: Vec<(Range<usize>, String)> = doc
            .inlines
            .iter()
            .filter_map(|i| match &i.kind {
                InlineKind::Link { url, .. } => Some((i.range.clone(), url.clone())),
                _ => None,
            })
            .collect();
        let links = links_to_chars(text, links);
        Rendered {
            links,
            job: layout_highlighted(text, &doc, style, width, highlighter),
            rules,
        }
    }
}

type JobCache = FrameCache<Rendered, JobComputer>;

/// The rendering of `text` with a highlighter, kept in `ui.data` under `id` until its key
/// (the text, style and width) changes, so that the highlighter runs once per change.
fn highlighted(
    ui: &Ui,
    id: Id,
    text: &str,
    style: &MarkdownStyle,
    wrap_width: f32,
    highlighter: &dyn Highlighter,
) -> Rendered {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (text, style, wrap_width.to_bits()).hash(&mut hasher);
    let key = hasher.finish();
    let slot = id.with("highlight");
    if let Some((k, rendered)) = ui.data(|d| d.get_temp::<(u64, Rendered)>(slot))
        && k == key
    {
        return rendered;
    }
    let rendered = render(text, style, wrap_width, Some(highlighter));
    ui.data_mut(|d| d.insert_temp(slot, (key, rendered.clone())));
    rendered
}

/// A Markdown editor, or a read-only view with the same look.
pub struct MarkdownEditor<'t> {
    text: &'t mut String,
    id: Option<Id>,
    read_only: bool,
    style: Option<MarkdownStyle>,
    highlighter: Option<&'t dyn Highlighter>,
    desired_rows: usize,
    accessible_label: String,
}

impl std::fmt::Debug for MarkdownEditor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownEditor")
            .field("text", &self.text)
            .field("id", &self.id)
            .field("read_only", &self.read_only)
            .field("style", &self.style)
            .field("highlighter", &self.highlighter.is_some())
            .field("desired_rows", &self.desired_rows)
            .field("accessible_label", &self.accessible_label)
            .finish()
    }
}

impl<'t> MarkdownEditor<'t> {
    /// An editor of `text`, which it edits in place.
    pub fn new(text: &'t mut String) -> Self {
        Self {
            text,
            id: None,
            read_only: false,
            style: None,
            highlighter: None,
            desired_rows: 8,
            accessible_label: String::from("Markdown"),
        }
    }

    /// The id of the `TextEdit`, to find its state before the editor shows; give the same
    /// one to a [`MarkdownToolbar`](crate::MarkdownToolbar).
    ///
    /// Without one, each editor takes the next automatic id of the `ui`, as a `TextEdit`
    /// does: apart from the others, but not kept if the widgets before it change.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// What screen readers call the field; defaults to `"Markdown"`, in English, so that
    /// it has a name at all.
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.accessible_label = label.into();
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

    /// Colour the code of fenced blocks that name a language (the first word of the info
    /// string) and YAML front matter with `highlighter`; other code stays plain. It runs when
    /// the text, the style or the width changes, not every frame; a highlighter swapped for
    /// another takes effect at the next such change.
    pub fn highlighter(mut self, highlighter: &'t dyn Highlighter) -> Self {
        self.highlighter = Some(highlighter);
        self
    }

    /// The height in rows of text; defaults to 8.
    pub fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    /// Shows the editor, taking its keys if it has the focus.
    pub fn show(self, ui: &mut Ui) -> MarkdownOutput {
        let id = self.id.unwrap_or_else(|| {
            // As `TextEdit` does with no id.
            let id = ui.next_auto_id();
            ui.skip_ahead_auto_ids(1);
            id
        });
        let takes_keys = !self.read_only && ui.is_enabled() && ui.memory(|m| m.has_focus(id));
        let edited = if takes_keys {
            take_keys(ui, id, self.text)
        } else {
            forget_composing(ui.ctx(), id);
            false
        };
        // Edited by the toolbar since the last frame.
        let edited_for_us = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<bool>(edited_id(id)))
            .unwrap_or(false);
        let style = self
            .style
            .unwrap_or_else(|| MarkdownStyle::from_visuals(ui.visuals()));
        // Set by each call of the layouter, so the last one (after any edit) wins.
        let mut rules = Vec::new();
        let mut links = Vec::new();
        let highlighter = self.highlighter;
        let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let rendered = match highlighter {
                Some(h) => highlighted(ui, id, buf.as_str(), &style, wrap_width, h),
                None => ui.memory_mut(|m| {
                    m.caches
                        .cache::<JobCache>()
                        .get((buf.as_str(), &style, wrap_width.to_bits()))
                        .clone()
                }),
            };
            rules = rendered.rules;
            links = rendered.links;
            ui.fonts_mut(|f| f.layout_job(rendered.job))
        };

        let mut text_edit = if self.read_only {
            let mut view: &str = self.text.as_str();
            TextEdit::multiline(&mut view)
                .id(id)
                .desired_rows(self.desired_rows)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter)
                .show(ui)
        } else {
            TextEdit::multiline(self.text)
                .id(id)
                .desired_rows(self.desired_rows)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter)
                .show(ui)
        };

        // Screen readers and tests find the field by this name.
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_label(self.accessible_label);
        });
        if edited {
            text_edit.response.mark_changed();
        }
        if edited || edited_for_us {
            // The `TextEdit` saw no change of its own, so it did not follow the cursor.
            scroll_to_cursor(ui, &text_edit);
        }
        paint_rules(ui, &text_edit, &rules);
        let link_clicked = link_at_pointer(ui, &text_edit, &links, self.read_only);
        MarkdownOutput {
            // A read-only view reports the keys it ignored as changes.
            changed: !self.read_only && text_edit.response.changed(),
            link_clicked,
            text_edit,
        }
    }
}

/// Scroll the primary cursor of `out` into view, as the `TextEdit` does after its own edits.
/// (Its cursor blink cannot be reset from outside egui.)
fn scroll_to_cursor(ui: &Ui, out: &TextEditOutput) {
    let galley = &out.galley;
    // From the state: `cursor_range` is `None` while the focus comes back from the toolbar.
    let Some(range) = out.state.cursor.range(galley) else {
        return;
    };
    let offset = out.galley_pos.to_vec2() - egui::vec2(galley.rect.left(), 0.0);
    let rect = galley.pos_from_cursor(range.primary).translate(offset);
    ui.scroll_to_rect(rect, None);
}

/// The destination of the link under the pointer where a click would report it (always when
/// `read_only`, else with the command key held): sets the pointing hand, and returns the link
/// if this frame's click is on it.
fn link_at_pointer(
    ui: &Ui,
    out: &TextEditOutput,
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
    let url = links
        .iter()
        .find(|(range, _)| range.contains(&at))
        .map(|(_, url)| url)?;
    ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
    // `clicked` is the primary button. A double or triple click is one more click of the
    // same gesture, already reported.
    (response.clicked() && !response.double_clicked() && !response.triple_clicked())
        .then(|| url.clone())
}

/// `links` with their byte ranges of `s` made char ranges, in one pass over `s`.
fn links_to_chars(s: &str, links: Vec<(Range<usize>, String)>) -> Vec<(Range<usize>, String)> {
    if links.is_empty() {
        return links;
    }
    let mut bytes: Vec<usize> = links.iter().flat_map(|(r, _)| [r.start, r.end]).collect();
    bytes.sort_unstable();
    bytes.dedup();
    let chars = byte_starts_to_chars(s, &bytes);
    let to_char = |b: usize| chars[bytes.binary_search(&b).expect("collected above")];
    links
        .into_iter()
        .map(|(r, url)| (to_char(r.start)..to_char(r.end), url))
        .collect()
}

/// The char indices of the byte offsets `starts`, in ascending order, in one pass over `s`.
fn byte_starts_to_chars(s: &str, starts: &[usize]) -> Vec<usize> {
    let mut chars = s.char_indices().map(|(b, _)| b).enumerate();
    starts
        .iter()
        .map(|&start| {
            chars
                .by_ref()
                .find(|&(_, b)| b >= start)
                .map_or(s.chars().count(), |(c, _)| c)
        })
        .collect()
}

/// The y, in the galley, of the middle of the row holding each char index of `rules`, in
/// ascending order: what `pos_from_cursor` gives, in one pass over the rows rather than one
/// per rule.
fn rule_ys(galley: &egui::Galley, rules: &[usize]) -> Vec<f32> {
    let mut ys = Vec::with_capacity(rules.len());
    let mut rules = rules.iter().peekable();
    let mut start = 0;
    for row in &galley.rows {
        let end = start + row.char_count_excluding_newline().0;
        while let Some(&&c) = rules.peek() {
            if c > end {
                break;
            }
            ys.push((row.min_y() + row.max_y()) / 2.0);
            rules.next();
        }
        start += row.char_count_including_newline().0;
    }
    ys
}

/// Draws a line through the row of each thematic break in view, given by char index.
fn paint_rules(ui: &Ui, out: &TextEditOutput, rules: &[usize]) {
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    let clip = out.text_clip_rect;
    let painter = ui.painter_at(clip);
    for y in rule_ys(&out.galley, rules) {
        let y = out.galley_pos.y + y;
        if clip.y_range().contains(y) {
            painter.hline(clip.x_range(), y, stroke);
        }
    }
}

/// What [`MarkdownEditor::show`] returns.
#[non_exhaustive]
pub struct MarkdownOutput {
    /// The text was edited this frame.
    pub changed: bool,
    /// A link was clicked, with its destination.
    pub link_clicked: Option<String>,
    /// What the `TextEdit` underneath returned: its response (with the editor's id), galley
    /// and cursor.
    pub text_edit: TextEditOutput,
}

impl std::fmt::Debug for MarkdownOutput {
    /// `TextEditOutput` is not `Debug`: only its response's id is shown.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownOutput")
            .field("changed", &self.changed)
            .field("link_clicked", &self.link_clicked)
            .field("id", &self.text_edit.response.id)
            .finish_non_exhaustive()
    }
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
        // Char ranges, not bytes.
        let r = JobComputer.compute(("日本 [x](u) [y](v)", &style, 100.0f32.to_bits()));
        assert_eq!(
            r.links,
            vec![(3..9, "u".to_string()), (10..16, "v".to_string())]
        );
    }

    #[test]
    fn the_rules_are_char_indices() {
        let style = MarkdownStyle::from_visuals(&egui::Visuals::dark());
        let r = JobComputer.compute(("日本\n\n---\n🎌\n\n***\n", &style, 100.0f32.to_bits()));
        assert_eq!(r.rules, vec![4, 11]);
    }

    #[test]
    fn rule_ys_match_the_cursor_positions() {
        let source = "# a\n\n---\n\nsome text that wraps over more than one row of the galley\n\n***\n\n- b\n\n___";
        let style = MarkdownStyle::from_visuals(&egui::Visuals::dark());
        let r = JobComputer.compute((source, &style, 60.0f32.to_bits()));
        assert_eq!(r.rules.len(), 3);
        let ctx = Context::default();
        let mut galley = None;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            galley = Some(ui.fonts_mut(|f| f.layout_job(r.job.clone())));
        });
        output.textures_delta.clear();
        let galley = galley.unwrap();
        assert!(galley.rows.len() > 6, "the text must wrap");
        let want: Vec<f32> = r
            .rules
            .iter()
            .map(|&c| galley.pos_from_cursor(CCursor::new(c)).center().y)
            .collect();
        assert_eq!(rule_ys(&galley, &r.rules), want);
    }

    #[test]
    fn out_of_range_clamps_to_the_end() {
        assert_eq!(char_to_byte("日本", 9), 6);
        assert_eq!(byte_to_char("日本", 99), 2);
        assert_eq!(char_to_byte("", 3), 0);
    }
}
