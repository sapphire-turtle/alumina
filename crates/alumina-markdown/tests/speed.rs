//! Timing for the parser on a document of about 10,000 lines.
//!
//! Ignored by default: it asserts nothing, because the numbers depend on the
//! machine. Run it with:
//! `cargo test -p alumina-markdown --release --test speed -- --ignored --nocapture`

use std::hint::black_box;
use std::time::Instant;

const SAMPLE: &str = include_str!("sample.md");
const TARGET_LINES: usize = 10_000;
const RUNS: u32 = 20;

#[test]
#[ignore]
fn parse_ten_thousand_lines() {
    let mut source = String::new();
    let mut lines = 0;
    while lines < TARGET_LINES {
        source.push_str(SAMPLE);
        lines += SAMPLE.lines().count();
    }

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
