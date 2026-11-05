//! [`CodeEditor`]: a monospace `TextEdit` that takes Enter, Tab, Backspace and the closing
//! brackets for the [`indent`](crate::indent) edits.

use std::ops::Range;
use std::sync::Arc;

use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::text_edit::{TextEditOutput, TextEditState};
use egui::{Context, Event, FocusDirection, Id, ImeEvent, Key, Modifiers, TextEdit, Ui};

use crate::indent::{Edit, Indent, backspace, close, indent, newline, outdent};

/// The byte offset of char `c` in `s`; past the end, `s.len()`.
fn char_to_byte(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b)
}

/// The char index of byte `b` in `s`; past the end, the char count.
fn byte_to_char(s: &str, b: usize) -> usize {
    s.char_indices().take_while(|(i, _)| *i < b).count()
}

/// The selection of the editor `id` in bytes of `text`; `None` if it has no cursor yet.
fn selection(ctx: &Context, id: Id, text: &str) -> Option<Range<usize>> {
    let range = TextEditState::load(ctx, id)?.cursor.char_range()?;
    let [start, end] = range.sorted_cursors();
    Some(char_to_byte(text, start.index.0)..char_to_byte(text, end.index.0))
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

/// A key the editor takes.
#[derive(Clone, Copy)]
enum Press {
    Enter,
    /// Tab, or Shift+Tab with `true`.
    Tab(bool),
    Backspace,
    /// A closing bracket typed.
    Closer(char),
}

/// The press `event` is, if the editor takes it. Keys count with exactly these modifiers: an
/// extra Alt is AltGr typing a char on Windows (Ctrl+Alt), an extra Command is a shortcut.
fn press(event: &Event) -> Option<Press> {
    match event {
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            let none = modifiers.matches_exact(Modifiers::NONE);
            match key {
                Key::Enter if none => Some(Press::Enter),
                Key::Backspace if none => Some(Press::Backspace),
                Key::Tab if none => Some(Press::Tab(false)),
                Key::Tab if modifiers.matches_exact(Modifiers::SHIFT) => Some(Press::Tab(true)),
                _ => None,
            }
        }
        Event::Text(t) => match t.as_str() {
            "}" => Some(Press::Closer('}')),
            "]" => Some(Press::Closer(']')),
            ")" => Some(Press::Closer(')')),
            _ => None,
        },
        _ => None,
    }
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

/// Whether `event`, left for egui, edits the text or moves the cursor.
fn egui_edits(event: &Event) -> bool {
    matches!(
        event,
        Event::Text(_)
            | Event::Paste(_)
            | Event::Cut
            | Event::Ime(_)
            | Event::Key { pressed: true, .. }
    )
}

/// Take the keys the focused, editable editor `id` handles out of this frame's input, and
/// apply their edits to `text` before the `TextEdit` sees the rest. Whether `text` changed.
///
/// The edits come before egui's, so the editor takes presses only up to the first event of
/// the frame that it leaves for egui and that edits the text or moves the cursor (a `Text`,
/// `Paste`, `Cut`, `Ime` event, or a pressed key it does not take, including a Backspace or
/// closer whose edit is `None`). From that event on it takes nothing more this frame: the
/// later presses reach egui as they are, so that the text comes out in the order typed.
fn take_keys(ui: &Ui, id: Id, text: &mut String, unit: Indent) -> bool {
    let ctx = ui.ctx();
    let was_composing = ctx.data(|d| d.get_temp::<bool>(composing_id(id)).unwrap_or(false));
    let (composing, ime) = ui.input_mut(|i| take_ime_enters(&mut i.events, was_composing));
    ctx.data_mut(|d| d.insert_temp(composing_id(id), composing));
    // A conversion in progress is the IME's: its keys and chars are not ours.
    if composing || ime {
        return false;
    }

    // The presses to act on, in order, up to the first event egui will handle that edits the
    // text or moves the cursor; most frames have none.
    let presses: Vec<(usize, Press)> = ui.input(|i| {
        let mut presses = Vec::new();
        for (at, e) in i.events.iter().enumerate() {
            match press(e) {
                Some(p) => presses.push((at, p)),
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
    let mut tabbed = false;
    let mut changed = false;
    // Each press that does not end the loop is taken, so `taken` is its index.
    for (taken, (at, press)) in presses.into_iter().enumerate() {
        let edit = match press {
            Press::Enter => Some(newline(text, sel.clone(), unit)),
            Press::Tab(false) => indent(text, sel.clone(), unit),
            Press::Tab(true) => outdent(text, sel.clone(), unit),
            Press::Backspace => backspace(text, sel.clone(), unit),
            Press::Closer(c) => close(text, sel.clone(), c, unit),
        };
        let is_tab = matches!(press, Press::Tab(_));
        // Nothing to do: the key is egui's, except Tab, whose own handling would delete the
        // selection.
        if edit.is_none() && !is_tab {
            // Left for egui, which edits at the cursor: what follows must come after it.
            break;
        }
        ui.input_mut(|i| i.events.remove(at - taken));
        tabbed |= is_tab;
        if let Some(edit) = edit {
            apply(ctx, id, text, &edit);
            sel = edit.selection;
            changed = true;
        }
    }
    if tabbed {
        // The Tab was ours: egui's focus must not move on with it.
        ui.memory_mut(|m| m.move_focus(FocusDirection::None));
    }
    changed
}

/// Follow the IME through `events`, starting `composing` or not, and drop each Enter
/// pressed while a composition is open: it commits the conversion. Eframe's native and web
/// integrations drop such an Enter themselves, but an integration that delivers an Enter key
/// with the commit would have egui turn it into a newline.
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

/// The layout of `text`: the one place the text gets its look.
fn layout_job(ui: &Ui, text: &str, wrap_width: f32) -> LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    // What a `TextEdit` draws its text in.
    let color = ui
        .visuals()
        .override_text_color
        .unwrap_or_else(|| ui.visuals().widgets.inactive.text_color());
    LayoutJob::simple(text.to_owned(), font, color, wrap_width)
}

/// A code editor, or a read-only view with the same look.
#[derive(Debug)]
pub struct CodeEditor<'t> {
    text: &'t mut String,
    id: Option<Id>,
    read_only: bool,
    indent: Indent,
    desired_rows: usize,
    accessible_label: String,
}

impl<'t> CodeEditor<'t> {
    /// An editor of `text`, which it edits in place.
    pub fn new(text: &'t mut String) -> Self {
        Self {
            text,
            id: None,
            read_only: false,
            indent: Indent::default(),
            desired_rows: 8,
            accessible_label: String::from("Code"),
        }
    }

    /// The id of the `TextEdit`, to find its state around the editor.
    ///
    /// Without one, each editor takes the next automatic id of the `ui`, as a `TextEdit`
    /// does: apart from the others, but not kept if the widgets before it change.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Selectable and copyable, but not editable.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// The unit of indentation of Enter, Tab, Shift+Tab and Backspace; defaults to
    /// [`Indent::default`]. It is the app's to choose and keep.
    pub fn indent(mut self, indent: Indent) -> Self {
        self.indent = indent;
        self
    }

    /// The height in rows of text; defaults to 8.
    pub fn desired_rows(mut self, rows: usize) -> Self {
        self.desired_rows = rows;
        self
    }

    /// What screen readers call the field; defaults to `"Code"`, in English, so that it has
    /// a name at all.
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.accessible_label = label.into();
        self
    }

    /// Shows the editor, taking its keys if it has the focus.
    pub fn show(self, ui: &mut Ui) -> CodeOutput {
        let id = self.id.unwrap_or_else(|| {
            // As `TextEdit` does with no id.
            let id = ui.next_auto_id();
            ui.skip_ahead_auto_ids(1);
            id
        });
        let takes_keys = !self.read_only && ui.is_enabled() && ui.memory(|m| m.has_focus(id));
        let edited = if takes_keys {
            take_keys(ui, id, self.text, self.indent)
        } else {
            forget_composing(ui.ctx(), id);
            false
        };
        let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let job = layout_job(ui, buf.as_str(), wrap_width);
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let rows = self.desired_rows;
        let mut text_edit = if self.read_only {
            let mut view: &str = self.text.as_str();
            show_edit(
                TextEdit::multiline(&mut view),
                id,
                true,
                rows,
                &mut layouter,
                ui,
            )
        } else {
            show_edit(
                TextEdit::multiline(self.text),
                id,
                false,
                rows,
                &mut layouter,
                ui,
            )
        };

        // Screen readers and tests find the field by this name.
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_label(self.accessible_label.clone());
        });
        if edited {
            text_edit.response.mark_changed();
            // The `TextEdit` saw no change of its own, so it did not follow the cursor.
            scroll_to_cursor(ui, &text_edit);
        }
        CodeOutput {
            // A read-only view reports the keys it ignored as changes.
            changed: !self.read_only && text_edit.response.changed(),
            text_edit,
        }
    }
}

/// Show `edit` with the look of a code editor.
fn show_edit<'a>(
    edit: TextEdit<'a>,
    id: Id,
    read_only: bool,
    rows: usize,
    layouter: &'a mut dyn FnMut(&Ui, &dyn egui::TextBuffer, f32) -> Arc<egui::Galley>,
    ui: &mut Ui,
) -> TextEditOutput {
    edit.id(id)
        .font(egui::TextStyle::Monospace)
        .lock_focus(!read_only)
        .desired_width(f32::INFINITY)
        .desired_rows(rows)
        .layouter(layouter)
        .show(ui)
}

/// Scroll the primary cursor of `out` into view, as the `TextEdit` does after its own edits.
/// (Its cursor blink cannot be reset from outside egui.)
fn scroll_to_cursor(ui: &Ui, out: &TextEditOutput) {
    let galley = &out.galley;
    let Some(range) = out.state.cursor.range(galley) else {
        return;
    };
    let offset = out.galley_pos.to_vec2() - egui::vec2(galley.rect.left(), 0.0);
    let rect = galley.pos_from_cursor(range.primary).translate(offset);
    ui.scroll_to_rect(rect, None);
}

/// What [`CodeEditor::show`] returns.
#[non_exhaustive]
pub struct CodeOutput {
    /// The text was edited this frame.
    pub changed: bool,
    /// What the `TextEdit` underneath returned: its response (with the editor's id), galley
    /// and cursor.
    pub text_edit: TextEditOutput,
}

impl std::fmt::Debug for CodeOutput {
    /// `TextEditOutput` is not `Debug`: only its response's id is shown.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeOutput")
            .field("changed", &self.changed)
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
        assert_eq!(char_to_byte("日本", 9), 6);
        assert_eq!(byte_to_char("日本", 99), 2);
    }
}
