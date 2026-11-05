//! The editor driven as a user would: focus, typing, IME, keys.

use alumina_code::egui::accesskit::Role;
use alumina_code::egui::text::{CCursor, CCursorRange};
use alumina_code::egui::text_edit::TextEditState;
use alumina_code::egui::{Event, Id, ImeEvent, Key, MissingGlyphPolicy, Modifiers};
use alumina_code::{CodeEditor, CodeTheme, Highlighter, Indent, TokenKind};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct State {
    text: String,
    changed: bool,
    read_only: bool,
    indent: Indent,
}

/// The editor's id in these tests, to set its selection.
const ID: &str = "code";

fn editor(text: &str, read_only: bool, indent: Indent) -> Harness<'static, State> {
    let mut h = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        |ui, s: &mut State| {
            // egui's fonts have no Japanese: draw tofu instead of panicking.
            ui.ctx().set_missing_glyph_policy(MissingGlyphPolicy::Tofu);
            let out = CodeEditor::new(&mut s.text)
                .id(Id::unique(ID))
                .read_only(s.read_only)
                .indent(s.indent)
                .show(ui);
            s.changed |= out.changed;
            // Something else for the focus to go to.
            let _ = ui.button("next");
        },
        State {
            text: text.into(),
            read_only,
            indent,
            ..State::default()
        },
    );
    h.run();
    h.get_by_role(Role::MultilineTextInput).click();
    h.run();
    h
}

fn with(text: &str) -> Harness<'static, State> {
    editor(text, false, Indent::Spaces(4))
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

fn set_cursor(h: &mut Harness<'_, State>, at: usize) {
    select(h, at, at);
}

fn key(h: &mut Harness<'_, State>, key: Key, modifiers: Modifiers) {
    h.input_mut().events.push(Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
    h.run();
}

fn text(h: &mut Harness<'_, State>, t: &str) {
    h.input_mut().events.push(Event::Text(t.into()));
    h.run();
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

fn preedit(t: &str) -> Event {
    Event::Ime(ImeEvent::Preedit {
        text: t.into(),
        active_range_chars: None,
    })
}

#[test]
fn enter_after_a_brace_goes_in() {
    let mut h = with("fn a() {}");
    set_cursor(&mut h, 8);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(h.state().text, "fn a() {\n    \n}");
    assert!(h.state().changed);
    text(&mut h, "x");
    assert_eq!(h.state().text, "fn a() {\n    x\n}");
}

#[test]
fn a_closer_comes_out() {
    let mut h = with("{\n        ");
    set_cursor(&mut h, 10);
    text(&mut h, "}");
    assert_eq!(h.state().text, "{\n    }");
}

#[test]
fn tab_and_shift_tab_over_lines() {
    let mut h = with("a\nb");
    select(&mut h, 0, 3);
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert_eq!(h.state().text, "    a\n    b");
    key(&mut h, Key::Tab, Modifiers::SHIFT);
    assert_eq!(h.state().text, "a\nb");
}

#[test]
fn tab_on_blank_lines_is_taken() {
    let mut h = with("\n\n");
    select(&mut h, 0, 2);
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert_eq!(h.state().text, "\n\n");
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(Id::unique(ID)));
}

#[test]
fn shift_tab_on_an_unindented_selection_keeps_it() {
    let mut h = with("ab");
    select(&mut h, 0, 2);
    key(&mut h, Key::Tab, Modifiers::SHIFT);
    assert_eq!(h.state().text, "ab");
}

#[test]
fn tab_keeps_the_focus() {
    let mut h = with("x");
    set_cursor(&mut h, 1);
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert_eq!(h.state().text, "x   ");
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(Id::unique(ID)));
}

#[test]
fn backspace_takes_a_level() {
    let mut h = with("        ");
    set_cursor(&mut h, 8);
    key(&mut h, Key::Backspace, Modifiers::NONE);
    assert_eq!(h.state().text, "    ");
}

#[test]
fn undo_reverts_one_edit() {
    let mut h = with("{}");
    set_cursor(&mut h, 1);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(h.state().text, "{\n    \n}");
    key(&mut h, Key::Enter, Modifiers::NONE);
    key(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(h.state().text, "{\n    \n}");
}

#[test]
fn an_egui_event_before_a_press_keeps_the_order() {
    let mut h = with("{}");
    set_cursor(&mut h, 1);
    h.input_mut()
        .events
        .extend([Event::Text("a".into()), enter_key()]);
    h.run();
    assert_eq!(h.state().text, "{a\n}");

    let mut h = with("{}");
    set_cursor(&mut h, 1);
    h.input_mut()
        .events
        .extend([enter_key(), Event::Text("x".into())]);
    h.run();
    assert_eq!(h.state().text, "{\n    x\n}");
}

#[test]
fn ime_keeps_enter_and_closers() {
    let mut h = with("{}");
    set_cursor(&mut h, 1);
    // One frame, as macOS and Windows may deliver them.
    h.input_mut().events.extend([
        preedit("か"),
        enter_key(),
        Event::Ime(ImeEvent::Commit("か".into())),
    ]);
    h.run();
    assert_eq!(h.state().text, "{か}");
}

/// An Enter before the IME's events in the frame is egui's as well.
#[test]
fn enter_in_a_frame_with_ime_is_egui_s() {
    let mut h = with("{}");
    set_cursor(&mut h, 1);
    h.input_mut().events.extend([enter_key(), preedit("か")]);
    h.run();
    assert_eq!(h.state().text, "{\nか}");
}

fn key_event(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn a_closer_comes_out_in_winit_s_and_the_web_s_order() {
    let k = key_event(Key::CloseCurlyBracket, Modifiers::SHIFT);
    for events in [
        vec![k.clone(), Event::Text("}".into())],
        vec![Event::Text("}".into()), k],
    ] {
        let mut h = with("{\n        ");
        set_cursor(&mut h, 10);
        h.input_mut().events.extend(events);
        h.run();
        assert_eq!(h.state().text, "{\n    }");
    }
}

#[test]
fn a_char_key_before_its_text_keeps_the_order() {
    let mut h = with("{}");
    set_cursor(&mut h, 1);
    h.input_mut().events.extend([
        key_event(Key::A, Modifiers::NONE),
        Event::Text("a".into()),
        enter_key(),
    ]);
    h.run();
    assert_eq!(h.state().text, "{a\n}");
}

#[test]
fn enter_at_the_end_of_a_crlf_line() {
    let mut h = with("a {\r\n}");
    set_cursor(&mut h, 4);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(h.state().text, "a {\r\n    \r\n}");
}

#[test]
fn read_only_takes_nothing() {
    let mut h = editor("{}", true, Indent::default());
    set_cursor(&mut h, 1);
    text(&mut h, "}");
    key(&mut h, Key::Enter, Modifiers::NONE);
    key(&mut h, Key::Tab, Modifiers::NONE);
    h.run();
    assert_eq!(h.state().text, "{}");
    assert!(!h.state().changed);
    // Tab leaves a read-only editor, as it does any widget.
    assert_ne!(h.ctx.memory(|m| m.focused()), Some(Id::unique(ID)));
}

#[test]
fn the_indent_unit_is_the_app_s() {
    let mut h = editor("{}", false, Indent::Tab);
    set_cursor(&mut h, 1);
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(h.state().text, "{\n\t\n}");
}

/// Marks every `fn` as a keyword for the language `"test"`, and counts its calls.
#[derive(Default)]
struct Fns {
    calls: AtomicUsize,
}

impl Highlighter for Fns {
    fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        (language == "test").then(|| {
            code.match_indices("fn")
                .map(|(i, m)| (i..i + m.len(), TokenKind::Keyword))
                .collect()
        })
    }
}

struct Hl {
    text: String,
    language: &'static str,
    highlighter: Option<Rc<Fns>>,
    galley: Option<Arc<alumina_code::egui::Galley>>,
}

fn highlighted(text: &str, language: &'static str, with: bool) -> (Harness<'static, Hl>, Rc<Fns>) {
    let fns = Rc::new(Fns::default());
    let mut h = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        |ui, s: &mut Hl| {
            let mut editor = CodeEditor::new(&mut s.text)
                .id(Id::unique(ID))
                .language(s.language);
            if let Some(f) = &s.highlighter {
                editor = editor.highlighter(f.as_ref());
            }
            s.galley = Some(editor.show(ui).text_edit.galley);
        },
        Hl {
            text: text.into(),
            language,
            highlighter: with.then(|| fns.clone()),
            galley: None,
        },
    );
    h.run();
    (h, fns)
}

/// The colour of each section of the editor's galley.
fn job_colors(h: &Harness<'_, Hl>) -> Vec<alumina_code::egui::Color32> {
    let galley = h.state().galley.as_ref().expect("shown");
    galley.job.sections.iter().map(|s| s.format.color).collect()
}

#[test]
fn a_highlighter_colours_the_text() {
    let (h, _) = highlighted("fn a", "test", true);
    let colors = job_colors(&h);
    let kw =
        CodeTheme::from_visuals(&alumina_code::egui::Visuals::dark()).color(TokenKind::Keyword);
    assert!(colors.contains(&kw), "{colors:?}");
}

#[test]
fn an_unknown_language_is_plain() {
    let (h, _) = highlighted("fn a", "other", true);
    let (plain, _) = highlighted("fn a", "other", false);
    assert_eq!(job_colors(&h), job_colors(&plain));
}

#[test]
fn the_highlighter_runs_once_for_unchanged_text() {
    let (mut h, fns) = highlighted("fn a", "test", true);
    for _ in 0..10 {
        h.run();
    }
    assert_eq!(fns.calls.load(Ordering::Relaxed), 1);
    h.state_mut().text.push('b');
    for _ in 0..3 {
        h.run();
    }
    assert_eq!(fns.calls.load(Ordering::Relaxed), 2);
}

#[test]
fn empty_text_keeps_its_row_height() {
    let (h, _) = highlighted("", "test", true);
    let (plain, _) = highlighted("", "test", false);
    let height = |h: &Harness<'_, Hl>| h.state().galley.as_ref().unwrap().rect.height();
    assert!(height(&h) > 0.0);
    assert_eq!(height(&h), height(&plain));
}
