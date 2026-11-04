//! The editor driven as a user would: focus, typing, IME, shortcuts.
//!
//! With `ALUMINA_RENDER_DIR` set, each test also renders its window there as a PNG.

use alumina_markdown::egui::accesskit::Role;
use alumina_markdown::egui::text::{CCursor, CCursorRange};
use alumina_markdown::egui::text_edit::TextEditState;
use alumina_markdown::egui::{
    Event, Id, ImageSource, ImeEvent, Key, MissingGlyphPolicy, Modifiers, TextEdit,
};
use alumina_markdown::{MarkdownEditor, MarkdownToolbar, ToolbarIcons, ToolbarLabels};
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

const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path d="M3 2h6a3 3 0 0 1 0 6H3z" fill="none" stroke="currentColor"/></svg>"#;

#[derive(Default)]
struct BarState {
    text: String,
    labels: Option<ToolbarLabels>,
    icons: Option<ToolbarIcons>,
    changed: bool,
}

fn with_toolbar(
    text: &str,
    labels: Option<ToolbarLabels>,
    icons: Option<ToolbarIcons>,
) -> Harness<'static, BarState> {
    let mut h = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, s: &mut BarState| {
            ui.ctx().set_missing_glyph_policy(MissingGlyphPolicy::Tofu);
            egui_extras::install_image_loaders(ui.ctx());
            let id = Id::unique(ID);
            let mut bar = MarkdownToolbar::new(&mut s.text, id);
            if let Some(l) = s.labels.clone() {
                bar = bar.labels(l);
            }
            if let Some(i) = s.icons.clone() {
                bar = bar.icons(i);
            }
            s.changed |= bar.show(ui).changed();
            MarkdownEditor::new(&mut s.text).id(id).show(ui);
        },
        BarState {
            text: text.to_owned(),
            labels,
            icons,
            changed: false,
        },
    );
    h.get_by_role(Role::MultilineTextInput).click();
    h.run();
    h
}

fn bar_selection(h: &Harness<'_, BarState>) -> std::ops::Range<usize> {
    let state = TextEditState::load(&h.ctx, Id::unique(ID)).expect("a state");
    let [a, b] = state
        .cursor
        .char_range()
        .expect("a cursor")
        .sorted_cursors();
    a.index.0..b.index.0
}

fn select_bar(h: &mut Harness<'_, BarState>, start: usize, end: usize) {
    let ctx = h.ctx.clone();
    let id = Id::unique(ID);
    let mut state = TextEditState::load(&ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(start),
        CCursor::new(end),
    )));
    state.store(&ctx, id);
    h.run();
}

#[test]
fn toolbar_strong_wraps_the_selection_and_refocuses() {
    let mut h = with_toolbar("a b", None, None);
    select_bar(&mut h, 2, 3);
    h.get_by_label("B").click();
    h.run();
    assert_eq!(h.state().text, "a **b**");
    assert!(h.state().changed);
    assert_eq!(bar_selection(&h), 4..5);
    assert!(h.ctx.memory(|m| m.has_focus(Id::unique(ID))));
}

#[test]
fn toolbar_labels_are_replaceable() {
    let labels = ToolbarLabels {
        strong: "太字".into(),
        ..ToolbarLabels::default()
    };
    let mut h = with_toolbar("a b", Some(labels), None);
    select_bar(&mut h, 2, 3);
    h.get_by_label("太字").click();
    h.run();
    assert_eq!(h.state().text, "a **b**");
}

#[test]
fn toolbar_icon_button_is_found_by_its_label() {
    let icons = ToolbarIcons {
        strong: Some(ImageSource::Bytes {
            uri: "bytes://b.svg".into(),
            bytes: SVG.as_bytes().into(),
        }),
        ..ToolbarIcons::default()
    };
    let mut h = with_toolbar("a b", None, Some(icons));
    select_bar(&mut h, 2, 3);
    h.get_by_label("B").click();
    h.run();
    assert_eq!(h.state().text, "a **b**");
}

#[test]
fn toolbar_heading_cycles() {
    let mut h = with_toolbar("a", None, None);
    for want in ["# a", "## a", "### a", "a"] {
        h.get_by_label("H").click();
        h.run();
        assert_eq!(h.state().text, want);
    }
}

#[test]
fn toolbar_acts_at_the_end_with_no_cursor() {
    let mut h = with_toolbar("a", None, None);
    h.ctx
        .data_mut(|d| d.remove::<TextEditState>(Id::unique(ID)));
    h.get_by_label("-").click();
    h.run();
    assert_eq!(h.state().text, "- a");
}

/// A toolbar and editor in the given order; the selection is set in chars.
fn ordered(below: bool, text: &str, sel: (usize, usize)) -> Harness<'static, BarState> {
    let mut h = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, s: &mut BarState| {
            ui.ctx().set_missing_glyph_policy(MissingGlyphPolicy::Tofu);
            let id = Id::unique(ID);
            if !s.changed {
                MarkdownToolbar::new(&mut s.text, id).show(ui);
            }
            MarkdownEditor::new(&mut s.text).id(id).show(ui);
            if s.changed {
                MarkdownToolbar::new(&mut s.text, id).show(ui);
            }
        },
        BarState {
            text: text.to_owned(),
            changed: below,
            ..BarState::default()
        },
    );
    h.get_by_role(Role::MultilineTextInput).click();
    h.run();
    select_bar(&mut h, sel.0, sel.1);
    h
}

/// Press, wait, release: as a mouse does.
fn slow_click(h: &mut Harness<'_, BarState>, label: &str) {
    let pos = h.get_by_label(label).rect().center();
    h.hover_at(pos);
    h.run();
    let button = |pressed| Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.event(button(true));
    h.step();
    h.step();
    h.event(button(false));
    h.step();
    h.run();
}

#[test]
fn toolbar_clicked_slowly_keeps_the_selection_above_or_below() {
    for below in [false, true] {
        let mut h = ordered(below, "a b", (2, 3));
        slow_click(&mut h, "B");
        assert_eq!(h.state().text, "a **b**", "below={below}");
        assert_eq!(bar_selection(&h), 4..5, "below={below}");
        assert!(h.ctx.memory(|m| m.has_focus(Id::unique(ID))));
    }
}

#[test]
fn toolbar_wraps_japanese_and_emoji() {
    for below in [false, true] {
        let mut h = ordered(below, "日本語🎌x", (1, 4));
        slow_click(&mut h, "B");
        assert_eq!(h.state().text, "日**本語🎌**x", "below={below}");
        assert_eq!(bar_selection(&h), 3..6, "below={below}");
    }
}

#[test]
fn toolbar_survives_the_app_replacing_the_text() {
    let mut h = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, s: &mut BarState| {
            let id = Id::unique(ID);
            MarkdownToolbar::new(&mut s.text, id).show(ui);
            MarkdownEditor::new(&mut s.text).id(id).show(ui);
            if ui.button("Clear").clicked() {
                s.text.clear();
            }
        },
        BarState {
            text: "hello world".into(),
            ..BarState::default()
        },
    );
    h.get_by_role(Role::MultilineTextInput).click();
    h.run();
    select_bar(&mut h, 11, 11);
    h.get_by_label("Clear").click();
    h.run();
    assert_eq!(h.state().text, "");
}

#[test]
fn toolbar_undo_restores_the_selection() {
    let mut h = ordered(true, "a b", (2, 3));
    slow_click(&mut h, "B");
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(h.state().text, "a b");
    assert_eq!(bar_selection(&h), 2..3);
}
