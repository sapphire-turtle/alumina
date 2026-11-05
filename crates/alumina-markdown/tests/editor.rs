//! The editor driven as a user would: focus, typing, IME, shortcuts.
//!
//! With `ALUMINA_RENDER_DIR` set, each test also renders its window there as a PNG.

use alumina_markdown::egui::accesskit::Role;
use alumina_markdown::egui::text::{CCursor, CCursorRange};
use alumina_markdown::egui::text_edit::TextEditState;
use alumina_markdown::egui::{
    Event, Id, ImageSource, ImeEvent, Key, MissingGlyphPolicy, Modifiers, PointerButton, Pos2,
    TextEdit,
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
    link: Option<String>,
    galley: Option<std::sync::Arc<alumina_markdown::egui::Galley>>,
    galley_pos: Pos2,
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
            if out.link_clicked.is_some() {
                s.link = out.link_clicked;
            }
            s.galley = Some(out.text_edit.galley.clone());
            s.galley_pos = out.text_edit.galley_pos;
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
    h.get_by_label("Bold").click();
    h.run();
    assert_eq!(h.state().text, "a **b**");
    assert!(h.state().changed);
    assert_eq!(bar_selection(&h), 4..5);
    assert!(h.ctx.memory(|m| m.has_focus(Id::unique(ID))));
}

#[test]
fn toolbar_labels_are_replaceable() {
    let mut labels = ToolbarLabels::default();
    labels.strong = "太字".into();
    let mut h = with_toolbar("a b", Some(labels), None);
    select_bar(&mut h, 2, 3);
    // The label is what shows; the name is what is found.
    assert!(h.query_by_label("太字").is_none());
    h.get_by_label("Bold").click();
    h.run();
    assert_eq!(h.state().text, "a **b**");
}

#[test]
fn toolbar_icon_button_is_found_by_its_label() {
    let mut icons = ToolbarIcons::default();
    icons.strong = Some(ImageSource::Bytes {
        uri: "bytes://b.svg".into(),
        bytes: SVG.as_bytes().into(),
    });
    let mut h = with_toolbar("a b", None, Some(icons));
    select_bar(&mut h, 2, 3);
    h.get_by_label("Bold").click();
    h.run();
    assert_eq!(h.state().text, "a **b**");
}

#[test]
fn toolbar_heading_cycles() {
    let mut h = with_toolbar("a", None, None);
    for want in ["# a", "## a", "### a", "a"] {
        h.get_by_label("Heading").click();
        h.run();
        assert_eq!(h.state().text, want);
    }
}

#[test]
fn toolbar_acts_at_the_end_with_no_cursor() {
    let mut h = with_toolbar("a", None, None);
    h.ctx
        .data_mut(|d| d.remove::<TextEditState>(Id::unique(ID)));
    h.get_by_label("Bulleted list").click();
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
        slow_click(&mut h, "Bold");
        assert_eq!(h.state().text, "a **b**", "below={below}");
        assert_eq!(bar_selection(&h), 4..5, "below={below}");
        assert!(h.ctx.memory(|m| m.has_focus(Id::unique(ID))));
    }
}

#[test]
fn toolbar_wraps_japanese_and_emoji() {
    for below in [false, true] {
        let mut h = ordered(below, "日本語🎌x", (1, 4));
        slow_click(&mut h, "Bold");
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
    slow_click(&mut h, "Bold");
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run();
    assert_eq!(h.state().text, "a b");
    assert_eq!(bar_selection(&h), 2..3);
}

/// The screen position of the middle of the cursor rect at char `at`.
fn char_pos(h: &Harness<'_, State>, at: usize) -> Pos2 {
    let s = h.state();
    let galley = s.galley.as_ref().expect("a galley");
    s.galley_pos + galley.pos_from_cursor(CCursor::new(at)).center().to_vec2()
}

/// Click at `pos` holding `modifiers`, over three frames as a pointer does.
fn click_at(h: &mut Harness<'_, State>, pos: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(pos));
    h.run();
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        h.run();
    }
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

#[test]
fn read_only_click_reports_link() {
    let mut h = editor_with("[x](https://e.x)", true);
    let pos = char_pos(&h, 1);
    click_at(&mut h, pos, Modifiers::NONE);
    assert_eq!(h.state().link.as_deref(), Some("https://e.x"));
}

#[test]
fn editing_plain_click_places_cursor() {
    let mut h = editor_with("[x](https://e.x)", false);
    let pos = char_pos(&h, 1);
    click_at(&mut h, pos, Modifiers::NONE);
    assert_eq!(h.state().link, None);
    assert_eq!(selection(&h), 1..1);
}

#[test]
fn editing_command_click_reports_link() {
    let mut h = editor_with("[x](https://e.x)", false);
    let pos = char_pos(&h, 1);
    click_at(&mut h, pos, Modifiers::COMMAND);
    assert_eq!(h.state().link.as_deref(), Some("https://e.x"));
}

#[test]
fn click_outside_a_link() {
    let mut h = editor_with("plain [x](https://e.x)", true);
    let pos = char_pos(&h, 2);
    click_at(&mut h, pos, Modifiers::NONE);
    assert_eq!(h.state().link, None);
}

#[test]
fn click_right_of_a_line_ending_link() {
    let mut h = editor_with("[x](https://e.x)\nnext", true);
    let end = char_pos(&h, 16);
    click_at(
        &mut h,
        end + alumina_markdown::egui::vec2(100.0, 0.0),
        Modifiers::NONE,
    );
    assert_eq!(h.state().link, None);
    // Control: on the link itself it does report.
    let on = char_pos(&h, 3);
    click_at(&mut h, on, Modifiers::NONE);
    assert_eq!(h.state().link.as_deref(), Some("https://e.x"));
}

#[test]
fn read_only_copies() {
    use alumina_markdown::egui::OutputCommand;
    let mut h = editor_with("# a", true);
    h.event(key(Modifiers::COMMAND, Key::A));
    h.step();
    h.event(Event::Copy);
    h.step();
    let copied = h
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|c| match c {
            OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        });
    assert_eq!(copied.as_deref(), Some("# a"));
}

#[derive(Default)]
struct TwoState {
    a: String,
    b: String,
}

#[test]
fn two_editors_without_an_id_are_apart() {
    let mut h = Harness::builder().with_size([400.0, 400.0]).build_ui_state(
        |ui, s: &mut TwoState| {
            ui.ctx().set_missing_glyph_policy(MissingGlyphPolicy::Tofu);
            MarkdownEditor::new(&mut s.a).desired_rows(3).show(ui);
            MarkdownEditor::new(&mut s.b).desired_rows(3).show(ui);
        },
        TwoState::default(),
    );
    h.run();
    h.get_all_by_role(Role::MultilineTextInput)
        .next()
        .expect("an editor")
        .click();
    h.run();
    h.get_all_by_role(Role::MultilineTextInput)
        .next()
        .expect("an editor")
        .type_text("x");
    h.run();
    assert_eq!(h.state().a, "x");
    assert_eq!(h.state().b, "");
}

#[derive(Default)]
struct ScrollState {
    text: String,
    cursor: Option<alumina_markdown::egui::Rect>,
    clip: Option<alumina_markdown::egui::Rect>,
}

#[test]
fn enter_at_the_bottom_scrolls_to_the_cursor() {
    let mut text: String = (0..40).map(|i| format!("line {i}\n")).collect();
    text.push_str("- a");
    let mut h = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        |ui, s: &mut ScrollState| {
            alumina_markdown::egui::ScrollArea::vertical().show(ui, |ui| {
                let out = MarkdownEditor::new(&mut s.text).id(Id::unique(ID)).show(ui);
                let te = &out.text_edit;
                s.cursor = te.cursor_range.map(|r| {
                    te.galley
                        .pos_from_cursor(r.primary)
                        .translate(te.galley_pos.to_vec2())
                });
                s.clip = Some(ui.clip_rect());
            });
        },
        ScrollState {
            text,
            ..ScrollState::default()
        },
    );
    h.run();
    h.ctx.memory_mut(|m| m.request_focus(Id::unique(ID)));
    h.run();
    let n = h.state().text.chars().count();
    let ctx = h.ctx.clone();
    let mut state = TextEditState::load(&ctx, Id::unique(ID)).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(n))));
    state.store(&ctx, Id::unique(ID));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().text.ends_with("- a\n- "), "{:?}", h.state().text);
    let (cursor, clip) = (h.state().cursor.unwrap(), h.state().clip.unwrap());
    assert!(
        clip.contains_rect(cursor),
        "cursor {cursor:?} outside clip {clip:?}"
    );
}

#[test]
fn toolbar_from_the_keyboard_acts_on_the_selection() {
    let mut h = with_toolbar("aa\nbb\ncc", None, None);
    select_bar(&mut h, 0, 8);
    // The code-block button is the last before the editor.
    h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    h.run();
    assert!(
        !h.ctx.memory(|m| m.has_focus(Id::unique(ID))),
        "focus left the editor"
    );
    h.key_press(Key::Space);
    h.run();
    assert_eq!(h.state().text, "```\naa\nbb\ncc\n```");
}

#[derive(Default)]
struct CountState {
    text: String,
    links: usize,
}

#[test]
fn double_click_reports_the_link_once() {
    let mut h = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        |ui, s: &mut CountState| {
            let out = MarkdownEditor::new(&mut s.text)
                .id(Id::unique(ID))
                .read_only(true)
                .show(ui);
            s.links += usize::from(out.link_clicked.is_some());
        },
        CountState {
            text: "[x](https://e.x)".into(),
            links: 0,
        },
    );
    h.run();
    let rect = h.get_by_role(Role::MultilineTextInput).rect();
    // On the `x`, near the left of the first row.
    let pos = Pos2::new(rect.left() + 10.0, rect.top() + 10.0);
    h.event(Event::PointerMoved(pos));
    h.run();
    // Four frames 50 ms apart: well within a double click.
    let start = h.ctx.input(|i| i.time) + 1.0;
    for (n, pressed) in [true, false, true, false].into_iter().enumerate() {
        h.input_mut().time = Some(start + 0.05 * n as f64);
        h.input_mut().events.push(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run();
    assert_eq!(h.state().links, 1);
}

#[test]
fn toolbar_heading_reads_an_indented_heading() {
    let mut h = with_toolbar("  # a", None, None);
    h.get_by_label("Heading").click();
    h.run();
    assert_eq!(h.state().text, "  ## a");
}

#[test]
fn editor_takes_an_accessible_label() {
    let mut h = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        |ui, s: &mut String| {
            MarkdownEditor::new(s).accessible_label("Note").show(ui);
        },
        String::new(),
    );
    h.run();
    h.get_by_role_and_label(Role::MultilineTextInput, "Note");
}

#[test]
fn toolbar_names_are_replaceable() {
    let mut h = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, s: &mut String| {
            let id = Id::unique(ID);
            let mut names = alumina_markdown::ToolbarNames::default();
            names.strong = "Fett".into();
            MarkdownToolbar::new(s, id).names(names).show(ui);
            MarkdownEditor::new(s).id(id).show(ui);
        },
        "a".to_string(),
    );
    h.run();
    h.get_by_label("Fett").click();
    h.run();
    assert_eq!(h.state(), "**a**");
}

#[test]
fn toolbar_edit_scrolls_to_the_cursor() {
    let mut text: String = (0..40).map(|i| format!("line {i}\n\n")).collect();
    text.push('a');
    let mut h = Harness::builder().with_size([600.0, 200.0]).build_ui_state(
        |ui, s: &mut ScrollState| {
            MarkdownToolbar::new(&mut s.text, Id::unique(ID)).show(ui);
            alumina_markdown::egui::ScrollArea::vertical().show(ui, |ui| {
                let out = MarkdownEditor::new(&mut s.text).id(Id::unique(ID)).show(ui);
                let te = &out.text_edit;
                s.cursor = te.cursor_range.map(|r| {
                    te.galley
                        .pos_from_cursor(r.primary)
                        .translate(te.galley_pos.to_vec2())
                });
                s.clip = Some(ui.clip_rect());
            });
        },
        ScrollState {
            text,
            ..ScrollState::default()
        },
    );
    h.run();
    h.ctx.memory_mut(|m| m.request_focus(Id::unique(ID)));
    h.run();
    let n = h.state().text.chars().count();
    let ctx = h.ctx.clone();
    let mut state = TextEditState::load(&ctx, Id::unique(ID)).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(n))));
    state.store(&ctx, Id::unique(ID));
    h.run();
    h.get_by_label("Bulleted list").click();
    h.run();
    assert!(h.state().text.ends_with("\n- a"), "{:?}", h.state().text);
    let (cursor, clip) = (h.state().cursor.unwrap(), h.state().clip.unwrap());
    assert!(
        clip.contains_rect(cursor),
        "cursor {cursor:?} outside clip {clip:?}"
    );
}

mod highlighting {
    use std::ops::Range;
    use std::sync::{Arc, Mutex};

    use alumina_markdown::egui::epaint::text::ByteRangeExt as _;
    use alumina_markdown::egui::{Color32, Id};
    use alumina_markdown::{
        CodeTheme, Highlighter, MarkdownEditor, MarkdownStyle, TokenKind, egui,
    };
    use egui_kittest::Harness;

    /// `fn` is a `Keyword` in `rust`, each `key` before `:` a `Key` in `yaml`; it records the
    /// languages it is asked.
    #[derive(Default)]
    struct Test {
        asked: Mutex<Vec<String>>,
    }

    impl Highlighter for Test {
        fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
            self.asked.lock().unwrap().push(language.to_owned());
            match language {
                "rust" => Some(
                    code.match_indices("fn")
                        .map(|(i, m)| (i..i + m.len(), TokenKind::Keyword))
                        .collect(),
                ),
                "yaml" => Some(
                    code.match_indices(':')
                        .map(|(i, _)| {
                            let start = code[..i].rfind('\n').map_or(0, |n| n + 1);
                            (start..i, TokenKind::Key)
                        })
                        .collect(),
                ),
                _ => None,
            }
        }
    }

    struct Garbage;
    impl Highlighter for Garbage {
        #[allow(clippy::reversed_empty_ranges)]
        fn highlight(&self, _: &str, _: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
            Some(vec![(0..999, TokenKind::Keyword), (5..2, TokenKind::Key)])
        }
    }

    struct Nothing;
    impl Highlighter for Nothing {
        fn highlight(&self, _: &str, _: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
            None
        }
    }

    /// The job of the editor over `text` for some frames: its sections as (range, colour).
    fn sections(
        text: &str,
        highlighter: Option<&dyn Highlighter>,
        frames: usize,
    ) -> Vec<(Range<usize>, Color32)> {
        let out = Arc::new(Mutex::new(Vec::new()));
        let sink = out.clone();
        let id = Id::unique("hl");
        let mut h = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
            move |ui, text: &mut String| {
                let mut e = MarkdownEditor::new(text).id(id);
                if let Some(h) = highlighter {
                    e = e.highlighter(h);
                }
                let o = e.show(ui);
                *sink.lock().unwrap() = o
                    .text_edit
                    .galley
                    .job
                    .sections
                    .iter()
                    .map(|s| (s.byte_range.as_usize(), s.format.color))
                    .collect();
            },
            text.to_owned(),
        );
        for _ in 0..frames {
            h.step();
        }
        out.lock().unwrap().clone()
    }

    fn color_at(sections: &[(Range<usize>, Color32)], byte: usize) -> Color32 {
        sections
            .iter()
            .find(|(r, _)| r.contains(&byte))
            .expect("a section")
            .1
    }

    fn style() -> MarkdownStyle {
        MarkdownStyle::from_visuals(&egui::Visuals::dark())
    }

    #[test]
    fn front_matter_is_highlighted_as_yaml() {
        let t = Test::default();
        let s = sections("---\ntitle: a\n---\n", Some(&t), 2);
        let style = style();
        assert_eq!(t.asked.lock().unwrap()[0], "yaml");
        for b in 4..9 {
            assert_eq!(color_at(&s, b), style.code_theme.color(TokenKind::Key));
        }
        assert_eq!(color_at(&s, 0), style.mark_color);
        assert_eq!(color_at(&s, 13), style.mark_color);
        assert_eq!(color_at(&s, 10), style.text_color);
    }

    #[test]
    fn a_code_block_in_a_quote_lands_on_its_code() {
        let t = Test::default();
        let src = "> ```rust\n> fn a\n> ```\n";
        let s = sections(src, Some(&t), 2);
        let style = style();
        let at = src.find("fn").unwrap();
        let kw = style.code_theme.color(TokenKind::Keyword);
        assert_eq!(color_at(&s, at), kw);
        assert_eq!(color_at(&s, at + 1), kw);
        let gt = src.find("> fn").unwrap();
        assert_ne!(color_at(&s, gt), kw);
        assert_ne!(color_at(&s, gt + 1), kw);
        assert_ne!(color_at(&s, at + 2), kw);
    }

    #[test]
    fn the_language_is_the_first_word_of_the_info() {
        let t = Test::default();
        sections("```rust ignore\nfn\n```", Some(&t), 2);
        assert_eq!(t.asked.lock().unwrap()[0], "rust");
    }

    #[test]
    fn an_indented_block_is_not_highlighted() {
        let t = Test::default();
        sections("para\n\n    fn a\n", Some(&t), 2);
        sections("```\nfn a\n```\n", Some(&t), 2);
        assert!(t.asked.lock().unwrap().is_empty());
    }

    /// Asserts which of the bytes of `source` are `Keyword`-coloured: exactly `expected`.
    fn assert_keywords(source: &str, expected: Range<usize>) {
        let kw = style().code_theme.color(TokenKind::Keyword);
        let s = sections(source, Some(&Test::default()), 2);
        for b in 0..source.len() {
            assert_eq!(
                color_at(&s, b) == kw,
                expected.contains(&b),
                "byte {b} of {source:?}"
            );
        }
    }

    #[test]
    fn a_token_on_a_later_line_of_a_quote_lands_on_its_code() {
        let src = "> ```rust\n> x\n> fn a\n> ```\n";
        let at = src.find("fn").unwrap();
        assert_keywords(src, at..at + 2);
    }

    #[test]
    fn a_token_on_a_later_line_of_a_list_item_lands_on_its_code() {
        let src = "- a\n\n  ```rust\n  x\n  fn a\n  ```\n";
        let at = src.find("fn").unwrap();
        assert_keywords(src, at..at + 2);
    }

    #[test]
    fn a_token_in_a_crlf_block_leaves_the_returns() {
        let src = "```rust\r\nx\r\nfn a\r\n```\r\n";
        let at = src.find("fn").unwrap();
        assert_keywords(src, at..at + 2);
    }

    /// One token over all of the code.
    struct Whole;
    impl Highlighter for Whole {
        fn highlight(&self, _: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
            Some(vec![(0..code.len(), TokenKind::Keyword)])
        }
    }

    #[test]
    fn a_token_across_pieces_is_split_around_the_prefixes() {
        let src = "> ```rust\n> ab\n> cd\n> ```\n";
        let kw = style().code_theme.color(TokenKind::Keyword);
        let s = sections(src, Some(&Whole), 2);
        let ab = src.find("ab").unwrap();
        let cd = src.find("cd").unwrap();
        for b in [ab, ab + 1, cd, cd + 1] {
            assert_eq!(color_at(&s, b), kw, "byte {b}");
        }
        // The `> ` before `cd`, and the newline after `ab`, which is code too.
        assert_ne!(color_at(&s, cd - 1), kw);
        assert_ne!(color_at(&s, cd - 2), kw);
    }

    #[test]
    fn bad_ranges_do_not_panic() {
        let s = sections("```rust\nfn é\n```\n", Some(&Garbage), 2);
        assert!(!s.is_empty());
    }

    /// Fixed ranges, whatever the code.
    struct Fixed(Vec<(Range<usize>, TokenKind)>);
    impl Highlighter for Fixed {
        fn highlight(&self, _: &str, _: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
            Some(self.0.clone())
        }
    }

    #[test]
    fn off_boundary_ranges_are_dropped_and_overlaps_go_to_the_later() {
        let style = style();
        let (kw, key) = (
            style.code_theme.color(TokenKind::Keyword),
            style.code_theme.color(TokenKind::Key),
        );
        // Code `aéb\nçd`: a=0, é=1..3, b=3, newline=4, ç=5..7, d=7.
        let src = "```rust\naéb\nçd\n```\n";
        let at = src.find('a').unwrap();
        // 0..3 Keyword, overlapped by 1..4 Key: the later wins on 1..3, and 3..4 is Key too.
        // 6..8 starts inside `ç`: dropped, and nothing overwrites those bytes.
        let h = Fixed(vec![
            (0..3, TokenKind::Keyword),
            (1..4, TokenKind::Key),
            (6..8, TokenKind::Type),
        ]);
        let s = sections(src, Some(&h), 2);
        assert_eq!(color_at(&s, at), kw);
        assert_eq!(color_at(&s, at + 1), key);
        assert_eq!(color_at(&s, at + 2), key);
        assert_eq!(color_at(&s, at + 3), key);
        let plain = color_at(&s, at + 4);
        assert_ne!(plain, key);
        for b in at + 5..at + 8 {
            assert_eq!(color_at(&s, b), plain, "byte {b}");
        }
    }

    #[test]
    fn the_highlighter_runs_once_for_unchanged_text() {
        let t = Test::default();
        let id = Id::unique("once");
        let mut h = Harness::builder().with_size([400.0, 200.0]).build_ui_state(
            |ui, text: &mut String| {
                MarkdownEditor::new(text).id(id).highlighter(&t).show(ui);
            },
            "```rust\nfn\n```\n".to_owned(),
        );
        for _ in 0..6 {
            h.step();
        }
        assert_eq!(t.asked.lock().unwrap().len(), 1);
        h.state_mut().push('x');
        for _ in 0..3 {
            h.step();
        }
        assert_eq!(t.asked.lock().unwrap().len(), 2);
    }

    fn assert_auto<T: Send + Sync + std::panic::RefUnwindSafe>() {}

    #[test]
    fn the_editor_is_send_sync_and_ref_unwind_safe() {
        assert_auto::<MarkdownEditor<'static>>();
    }

    #[test]
    fn no_highlighter_looks_as_before() {
        let src = "# t\n---\na: 1\n---\n```rust\nfn\n```\n> ```rust\n> fn\n> ```\n";
        assert_eq!(sections(src, None, 2), sections(src, Some(&Nothing), 2));
    }

    #[test]
    fn the_theme_is_part_of_the_style() {
        let a = style();
        let mut b = style();
        b.code_theme.set_color(TokenKind::Key, Color32::RED);
        assert_eq!(
            a.code_theme,
            CodeTheme::from_visuals(&egui::Visuals::dark())
        );
        assert_ne!(a, b);
    }
}
