//! Timing for the parser and the editor on documents of about 10,000 and 1,000 lines.
//!
//! Ignored by default: it asserts nothing, because the numbers depend on the
//! machine. Run it with:
//! `cargo test -p alumina-markdown --release --test speed -- --ignored --nocapture`

use std::hint::black_box;
use std::time::{Duration, Instant};

const SAMPLE: &str = include_str!("sample.md");
const TARGET_LINES: usize = 10_000;
const RUNS: u32 = 20;

#[test]
#[ignore]
fn parse_ten_thousand_lines() {
    let source = document(TARGET_LINES);

    let doc = alumina_markdown::parse(&source);
    let style =
        alumina_markdown::MarkdownStyle::from_visuals(&alumina_markdown::egui::Visuals::dark());
    let start = Instant::now();
    for _ in 0..RUNS {
        black_box(alumina_markdown::layout(&source, &doc, &style, 600.0));
    }
    eprintln!("layout: mean {:?} per layout", start.elapsed() / RUNS);

    let start = Instant::now();
    for _ in 0..RUNS {
        black_box(alumina_markdown::parse(black_box(&source)));
    }
    let mean = start.elapsed() / RUNS;

    eprintln!(
        "parse of {} lines, {} bytes: mean {:?} per parse over {} runs",
        source.lines().count(),
        source.len(),
        mean,
        RUNS,
    );
}

/// The sample repeated to at least `target` lines.
fn document(target: usize) -> String {
    let mut source = String::new();
    while source.lines().count() < target {
        source.push_str(SAMPLE);
    }
    source
}

/// Mean frame time of `source` read-only in a `ScrollArea` of a 600 by 600 window: after a
/// one-char change, then with no change.
///
/// Changed frames each append a char, so the layout is made again; unchanged frames hit
/// the cache. `run` runs one frame of the editor over the text.
fn frames(mut source: String, mut run: impl FnMut(&mut String)) -> (Duration, Duration) {
    // The first frames lay it out and settle the scroll area.
    for _ in 0..3 {
        run(&mut source);
    }
    let start = Instant::now();
    for _ in 0..RUNS {
        run(&mut source);
    }
    let unchanged = start.elapsed() / RUNS;
    let start = Instant::now();
    for _ in 0..RUNS {
        source.push('x');
        run(&mut source);
    }
    (start.elapsed() / RUNS, unchanged)
}

/// Every ASCII word of the code is a keyword: cheap, and as many tokens as can be.
struct Words;

impl alumina_markdown::Highlighter for Words {
    fn highlight(
        &self,
        language: &str,
        code: &str,
    ) -> Option<Vec<(std::ops::Range<usize>, alumina_markdown::TokenKind)>> {
        if language != "rust" {
            return None;
        }
        let mut tokens = Vec::new();
        let mut start = None;
        for (i, b) in code.bytes().chain(std::iter::once(b' ')).enumerate() {
            match (b.is_ascii_alphanumeric(), start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    tokens.push((s..i, alumina_markdown::TokenKind::Keyword));
                    start = None;
                }
                _ => {}
            }
        }
        Some(tokens)
    }
}

fn editor(ui: &mut alumina_markdown::egui::Ui, text: &mut String) {
    editor_with(ui, text, None);
}

fn editor_with(
    ui: &mut alumina_markdown::egui::Ui,
    text: &mut String,
    highlighter: Option<&dyn alumina_markdown::Highlighter>,
) {
    use alumina_markdown::MarkdownEditor;
    use alumina_markdown::egui::{Id, MissingGlyphPolicy, ScrollArea};
    // egui's fonts have no Japanese: draw tofu instead of panicking.
    ui.ctx().set_missing_glyph_policy(MissingGlyphPolicy::Tofu);
    ScrollArea::vertical().show(ui, |ui| {
        let editor = MarkdownEditor::new(text)
            .id(Id::unique("big"))
            .read_only(true);
        match highlighter {
            Some(h) => editor.highlighter(h).show(ui),
            None => editor.show(ui),
        };
    });
}

fn report(what: &str, source: &str, (changed, unchanged): (Duration, Duration)) {
    eprintln!(
        "{what}, {} lines, {} bytes: mean {changed:?} per frame after a one-char change, \
         {unchanged:?} with no change, over {RUNS} frames each",
        source.lines().count(),
        source.len(),
    );
}

/// The documents of about 10,000 and 1,000 lines in the test harness, which turns AccessKit
/// on and reads its whole tree each frame.
#[test]
#[ignore]
fn layout_ten_thousand_lines() {
    use egui_kittest::Harness;

    for target in [TARGET_LINES, 1_000] {
        let source = document(target);
        let mut harness = Harness::builder()
            .with_size([600.0, 600.0])
            .build_ui_state(editor, String::new());
        let times = frames(source.clone(), |text| {
            std::mem::swap(harness.state_mut(), text);
            harness.step();
            std::mem::swap(harness.state_mut(), text);
        });
        report("harness", &source, times);
    }
}

/// The same frames on a bare `Context`, as an app without a screen reader runs them: no
/// AccessKit, and the frame's shapes tessellated.
#[test]
#[ignore]
fn frames_without_accesskit() {
    use alumina_markdown::egui::{Context, Pos2, RawInput, Rect, vec2};

    for target in [TARGET_LINES, 1_000] {
        let source = document(target);
        let ctx = Context::default();
        let times = frames(source.clone(), |text| {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 600.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| editor(ui, text));
            out.textures_delta.clear();
            black_box(ctx.tessellate(out.shapes, out.pixels_per_point));
        });
        report("bare context", &source, times);
    }
}

/// The bare-context frames with a highlighter that colours every word of the `rust` blocks.
#[test]
#[ignore]
fn frames_with_a_highlighter() {
    use alumina_markdown::egui::{Context, Pos2, RawInput, Rect, vec2};

    for target in [TARGET_LINES, 1_000] {
        let source = document(target);
        let ctx = Context::default();
        let times = frames(source.clone(), |text| {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 600.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| editor_with(ui, text, Some(&Words)));
            out.textures_delta.clear();
            black_box(ctx.tessellate(out.shapes, out.pixels_per_point));
        });
        report("bare context, highlighted", &source, times);
    }
}
