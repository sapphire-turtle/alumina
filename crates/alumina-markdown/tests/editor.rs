//! The editor driven as a user would: focus, typing, IME, shortcuts.
//!
//! With `ALUMINA_RENDER_DIR` set, each test also renders its window there as a PNG.

use alumina_markdown::MarkdownEditor;
use alumina_markdown::egui::accesskit::Role;
use alumina_markdown::egui::text::{CCursor, CCursorRange};
use alumina_markdown::egui::text_edit::TextEditState;
use alumina_markdown::egui::{Event, Id, ImeEvent, Key, MissingGlyphPolicy, Modifiers, TextEdit};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

#[derive(Default)]
struct State {
    text: String,
    changed: bool,
    sections: usize,
    read_only: bool,
}

fn render(harness: &mut Harness<'_, impl Sized>, name: &str) {
    let Ok(dir) = std::env::var("ALUMINA_RENDER_DIR") else {
        return;
    };
    harness.run();
    let image = harness.render().expect("render");
    image.save(format!("{dir}/{name}.png")).expect("save");
}

/// The editor's id in these tests, to set its selection.
const ID: &str = "md";

fn editor(read_only: bool) -> Harness<'static, State> {
    Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        |ui, s: &mut State| {
            // egui's fonts have no Japanese: draw tofu instead of panicking.
            ui.ctx().set_missing_glyph_policy(MissingGlyphPolicy::Tofu);
            let out = MarkdownEditor::new(&mut s.text)
                .id(Id::unique(ID))
                .read_only(s.read_only)
                .show(ui);
            s.changed |= out.changed;
            s.sections = out.text_edit.galley.job.sections.len();
        },
        State {
            read_only,
            ..State::default()
        },
    )
}

fn focus(h: &mut Harness<'_, State>) {
    h.get_by_role(Role::MultilineTextInput).click();
    h.run();
}

#[test]
fn types_and_reports_a_change() {
    let mut h = editor(false);
    focus(&mut h);
    h.get_by_role(Role::MultilineTextInput).type_text("# hi");
    h.run();
    assert_eq!(h.state().text, "# hi");
    assert!(h.state().changed);
    render(&mut h, "editor_heading");
}

#[test]
fn read_only_does_not_change() {
    let mut h = editor(true);
    focus(&mut h);
    h.get_by_role(Role::MultilineTextInput).type_text("# hi");
    h.run();
    assert_eq!(h.state().text, "");
    assert!(!h.state().changed);
}

#[test]
#[allow(deprecated)]
fn ime_composes_and_commits() {
    let mut h = editor(false);
    focus(&mut h);
    h.event(Event::Ime(ImeEvent::Enabled));
    h.event(Event::Ime(ImeEvent::Preedit {
        text: "にほ".into(),
        active_range_chars: None,
    }));
    h.run();
    h.event(Event::Ime(ImeEvent::Commit("日本".into())));
    h.run();
    assert_eq!(h.state().text, "日本");
}

#[test]
fn layout_follows_the_text() {
    let mut h = editor(false);
    focus(&mut h);
    h.get_by_role(Role::MultilineTextInput).type_text("**a**");
    h.run();
    assert_eq!(h.state().text, "**a**");
    assert!(h.state().sections > 1);
}

/// A focused editor holding `text`.
fn editor_with(text: &str, read_only: bool) -> Harness<'static, State> {
    let mut h = editor(read_only);
    h.state_mut().text = text.into();
    h.run();
    focus(&mut h);
    h
}

/// Select chars `start..end` of the editor, as egui stores it.
fn select(h: &mut Harness<'_, State>, start: usize, end: usize) {
    let ctx = h.ctx.clone();
    let id = Id::unique(ID);
    let mut state = TextEditState::load(&ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(start),
        CCursor::new(end),
    )));
    state.store(&ctx, id);
}

/// The selected chars of the editor.
fn selection(h: &Harness<'_, State>) -> std::ops::Range<usize> {
    let state = TextEditState::load(&h.ctx, Id::unique(ID)).expect("a state");
    let r = state.cursor.char_range().expect("a cursor");
    let [a, b] = r.sorted_cursors();
    a.index.0..b.index.0
}

fn end(h: &mut Harness<'_, State>) {
    let n = h.state().text.chars().count();
    select(h, n, n);
}

#[test]
fn ctrl_b_wraps() {
    let mut h = editor_with("a b", false);
    select(&mut h, 2, 3);
    h.key_press_modifiers(Modifiers::COMMAND, Key::B);
    h.run();
    assert_eq!(h.state().text, "a **b**");
    assert!(h.state().changed);
}

#[test]
fn shortcuts_wrap() {
    let cases = [
        (Modifiers::COMMAND, Key::I, "a *b*"),
        (Modifiers::COMMAND, Key::K, "a [b](url)"),
        (Modifiers::COMMAND | Modifiers::SHIFT, Key::X, "a ~~b~~"),
        (Modifiers::COMMAND, Key::E, "a `b`"),
    ];
    for (modifiers, key, want) in cases {
        let mut h = editor_with("a b", false);
        select(&mut h, 2, 3);
        h.key_press_modifiers(modifiers, key);
        h.run();
        assert_eq!(h.state().text, want, "{modifiers:?} {key:?}");
    }
}

#[test]
fn cut_is_not_taken() {
    let mut h = editor_with("a b", false);
    select(&mut h, 2, 3);
    h.key_press_modifiers(Modifiers::COMMAND, Key::X);
    h.event(Event::Cut);
    h.run();
    assert_eq!(h.state().text, "a ");
}

#[test]
fn shortcut_is_one_undo_step() {
    let mut h = editor_with("a ", false);
    end(&mut h);
    // Typed just now: egui has no undo point for it yet.
    h.get_by_role(Role::MultilineTextInput).type_text("b");
    h.run();
    select(&mut h, 2, 3);
    h.key_press_modifiers(Modifiers::COMMAND, Key::B);
    h.run();
    assert_eq!(h.state().text, "a **b**");
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(h.state().text, "a b");
}

#[test]
fn enter_continues_list() {
    let mut h = editor_with("- a", false);
    end(&mut h);
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "- a\n- ");
    assert_eq!(selection(&h), 6..6);
    assert!(h.state().changed);
}

#[test]
fn enter_outside_a_list_is_egui_s() {
    let mut h = editor_with("a", false);
    end(&mut h);
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "a\n");
}

#[test]
fn tab_indents() {
    let mut h = editor_with("- a", false);
    end(&mut h);
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().text, "  - a");
    // Still focused: Shift+Tab reaches the editor.
    h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    h.run();
    assert_eq!(h.state().text, "- a");
}

/// What Tab does to a plain `TextEdit` holding `text`.
fn plain_tab(text: &str) -> String {
    let mut h = Harness::builder()
        .with_size([400.0, 200.0])
        // A bare `TextEdit` has no label.
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, s: &mut String| {
                TextEdit::multiline(s).id(Id::unique(ID)).show(ui);
            },
            text.to_string(),
        );
    h.run();
    h.get_by_role(Role::MultilineTextInput).click();
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.state().clone()
}

#[test]
fn tab_outside_a_list_is_egui_s() {
    let mut h = editor_with("a", false);
    end(&mut h);
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().text, plain_tab("a"));
}

#[test]
fn japanese_selection_shortcut() {
    let mut h = editor_with("日本語", false);
    select(&mut h, 1, 2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::B);
    h.run();
    assert_eq!(h.state().text, "日**本**語");
    assert_eq!(selection(&h), 3..4);
}

fn enter_key() -> Event {
    Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn enter_during_ime_does_not_continue() {
    let mut h = editor_with("- a", false);
    end(&mut h);
    // One frame, as macOS and Windows may deliver them: `Harness::event` would run a
    // frame for each.
    h.input_mut().events.extend([
        Event::Ime(ImeEvent::Preedit {
            text: "か".into(),
            active_range_chars: None,
        }),
        enter_key(),
        Event::Ime(ImeEvent::Commit("か".into())),
    ]);
    h.run();
    assert_eq!(h.state().text, "- aか");
}

#[test]
fn enter_while_composing_does_not_continue() {
    let mut h = editor_with("- a", false);
    end(&mut h);
    h.event(Event::Ime(ImeEvent::Preedit {
        text: "か".into(),
        active_range_chars: None,
    }));
    h.run();
    h.event(enter_key());
    h.run();
    h.event(Event::Ime(ImeEvent::Commit("か".into())));
    h.run();
    assert_eq!(h.state().text, "- aか");
    // Composing is over: Enter continues the list again.
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "- aか\n- ");
}

#[test]
fn read_only_ignores_keys() {
    let mut h = editor_with("a b", true);
    select(&mut h, 2, 3);
    h.key_press_modifiers(Modifiers::COMMAND, Key::B);
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "a b");
    assert!(!h.state().changed);
}

fn key(modifiers: Modifiers, key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn altgr_typing_is_not_a_shortcut() {
    // Windows reports AltGr as Ctrl+Alt, and egui-winit falls back to the physical key.
    let mut h = editor_with("a", false);
    end(&mut h);
    h.input_mut().events.extend([
        key(Modifiers::COMMAND | Modifiers::ALT, Key::E),
        Event::Text("€".into()),
    ]);
    h.run();
    assert_eq!(h.state().text, "a€");
}

#[test]
fn extra_shift_is_not_the_shortcut() {
    let mut h = editor_with("a b", false);
    select(&mut h, 2, 3);
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::B);
    h.run();
    assert_eq!(h.state().text, "a b");
}

#[test]
fn composing_is_forgotten_when_not_editable() {
    let mut h = editor_with("- a", false);
    end(&mut h);
    h.event(Event::Ime(ImeEvent::Preedit {
        text: "か".into(),
        active_range_chars: None,
    }));
    h.run();
    // The commit never comes: the editor turns read-only for a frame, then back.
    h.state_mut().read_only = true;
    h.run();
    h.state_mut().read_only = false;
    h.run();
    end(&mut h);
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "- aか\n- ");
}

#[test]
fn typing_ends_a_composition() {
    let mut h = editor_with("- a", false);
    end(&mut h);
    h.event(Event::Ime(ImeEvent::Preedit {
        text: "か".into(),
        active_range_chars: None,
    }));
    h.run();
    h.event(Event::Ime(ImeEvent::Preedit {
        text: String::new(),
        active_range_chars: None,
    }));
    h.run();
    // A stale flag could also be ended by plain text.
    h.ctx
        .data_mut(|d| d.insert_temp(Id::unique(ID).with("composing"), true));
    h.event(Event::Text("b".into()));
    h.run();
    assert_eq!(h.state().text, "- ab");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "- ab\n- ");
}

#[test]
fn enter_with_no_cursor_acts_at_the_end() {
    let mut h = editor_with("- a", false);
    // Focused, but no stored cursor yet.
    h.ctx
        .data_mut(|d| d.remove::<TextEditState>(Id::unique(ID)));
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().text, "- a\n- ");
}
