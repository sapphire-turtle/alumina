//! The editor driven as a user would: focus, typing, IME.
//!
//! With `ALUMINA_RENDER_DIR` set, each test also renders its window there as a PNG.

use alumina_markdown::MarkdownEditor;
use alumina_markdown::egui::accesskit::Role;
use alumina_markdown::egui::{Event, ImeEvent, MissingGlyphPolicy};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

#[derive(Default)]
struct State {
    text: String,
    changed: bool,
    sections: usize,
}

fn render(harness: &mut Harness<'_, impl Sized>, name: &str) {
    let Ok(dir) = std::env::var("ALUMINA_RENDER_DIR") else {
        return;
    };
    harness.run();
    let image = harness.render().expect("render");
    image.save(format!("{dir}/{name}.png")).expect("save");
}

fn editor(read_only: bool) -> Harness<'static, State> {
    Harness::builder().with_size([400.0, 200.0]).build_ui_state(
        move |ui, s: &mut State| {
            // egui's fonts have no Japanese: draw tofu instead of panicking.
            ui.ctx().set_missing_glyph_policy(MissingGlyphPolicy::Tofu);
            let out = MarkdownEditor::new(&mut s.text)
                .read_only(read_only)
                .show(ui);
            s.changed |= out.changed;
            s.sections = out.text_edit.galley.job.sections.len();
        },
        State::default(),
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
