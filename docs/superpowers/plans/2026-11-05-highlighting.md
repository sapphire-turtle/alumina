# Syntax Highlighting and Front Matter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `Highlighter` trait in alumina-code that apps implement, used by `CodeEditor` and by alumina-markdown's code blocks and new YAML front matter, with two highlighters in the demo (YAML by hand, Rust over syntect).

**Architecture:** alumina-code defines `Highlighter`, `TokenKind` and `CodeTheme`, and colours a `LayoutJob` from the tokens; each editor caches its highlighted layout in egui memory under its id, keyed by a hash of what it depends on. alumina-markdown depends on alumina-code, records each code block's content pieces in the parser, adds `BlockKind::FrontMatter`, and paints tokens over its existing per-byte format pass. The demo implements the trait.

**Tech Stack:** Rust 2024, egui 0.37, pulldown-cmark 0.13 (`ENABLE_YAML_STYLE_METADATA_BLOCKS`), syntect 5 in the demo only, egui_kittest.

**Spec:** `docs/superpowers/specs/2026-11-05-highlighting-design.md`

## Global Constraints

- alumina-code depends on egui only (serde behind its feature); alumina-markdown on egui, pulldown-cmark and alumina-code. syntect only in `demo/Cargo.toml`.
- Every change to a published crate's API is additive: `MarkdownStyle`, `Block`, `BlockKind` are `#[non_exhaustive]`, so new fields and variants are fine; existing signatures (`layout`, `parse`, `MarkdownEditor`, `CodeEditor::new`) do not change. New public enums and output structs are `#[non_exhaustive]`; `#![warn(missing_docs)]` holds.
- Token ranges from a highlighter are untrusted: clamp to the code, drop empty, reversed or off-char-boundary ranges, paint in order (later wins); never panic.
- With no highlighter, or a highlighter returning `None`, output is exactly as before this plan (same `LayoutJob` sections).
- The highlighter runs only when the text, the language or the theme changes, not every frame.
- Commit messages: `feat(<crate>): … (#15)` / `(#14)`, ending with the `Co-Authored-By` line the session gives. `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass after every task.

## Review Focus

1. A code block inside a list or a quote: ranges from the highlighter land on the code's bytes, never on the `> ` or indentation prefixes. — test in Task 3.
2. Front matter while typing: a lone `---` at the start, or an unclosed one, must not swallow or break the document. — test in Task 2.
3. A highlighter returning garbage ranges (past the end, inside a multi-byte char, reversed, overlapping). — tests in Tasks 1 and 3.
4. Highlighting is cached: unchanged text over many frames calls the highlighter once. — tests in Tasks 1 and 3.
5. Dark and light themes both read: `CodeTheme::from_visuals` differs between them. — test in Task 1.

---

### Task 1: alumina-code — `Highlighter`, `TokenKind`, `CodeTheme`, and the editor's colours

**Files:**
- Create: `crates/alumina-code/src/highlight.rs`
- Modify: `crates/alumina-code/src/editor.rs`, `crates/alumina-code/src/lib.rs`, `crates/alumina-code/tests/editor.rs`, `crates/alumina-code/README.md`, `crates/alumina-code/CHANGELOG.md`

**Interfaces (produces, re-exported from lib.rs):**
- `pub trait Highlighter { fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>>; }`
- `#[non_exhaustive] #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum TokenKind { Comment, Keyword, Type, Function, String, Number, Constant, Key, Punctuation, Attribute }`
- `#[derive(Clone, Debug, PartialEq, Eq, Hash)] pub struct CodeTheme` with `pub fn from_visuals(&Visuals) -> Self`, `pub fn color(&self, TokenKind) -> Color32`, `pub fn set_color(&mut self, TokenKind, Color32)`; `Default` = `from_visuals(&Visuals::default())`.
- `pub fn highlighted_job(code: &str, tokens: &[(Range<usize>, TokenKind)], font: FontId, color: Color32, theme: &CodeTheme, wrap_width: f32) -> LayoutJob` — one section per run of equal colour; text outside tokens in `color`; ranges sanitised per Global Constraints. (alumina-markdown does not use it; it is the editor's, public so an app can lay out a read-only snippet itself.)
- `CodeEditor::language(impl Into<String>)`, `CodeEditor::highlighter(&'t dyn Highlighter)` (same lifetime as the text), `CodeEditor::theme(CodeTheme)`.

Colours of `from_visuals` (dark / light), `Comment` = `visuals.weak_text_color()`, `Punctuation` = `visuals.text_color()` in both:

| Kind | dark | light |
|---|---|---|
| Keyword | `#C678DD` | `#A626A4` |
| Type | `#E5C07B` | `#C18401` |
| Function | `#61AFEF` | `#4078F2` |
| String | `#98C379` | `#50A14F` |
| Number, Constant | `#D19A66` | `#986801` |
| Key | `#E06C75` | `#E45649` |
| Attribute | `#56B6C2` | `#0184BC` |

The editor: when a highlighter is given, the layouter returns the cached job for `hash((text, language, theme, wrap_width bits, font, text colour))`, kept in `ui.data` under `id.with("highlight")` as `(u64, Arc<LayoutJob>)`; on a miss it calls the highlighter, `highlighted_job` (or `layout_job`'s plain job when it returns `None`), and stores it. Without a highlighter it is `layout_job` as today.

- [ ] **Step 1: Failing tests.** Unit tests in `highlight.rs`: `highlighted_job("fn a", &[(0..2, Keyword)], …)` has two sections, the first `theme.color(Keyword)`, the second the given colour; garbage ranges `[(2..99, String), (3..1, Number), (1..2, Key)]` on `"日本"` (byte 1 is inside 日) do not panic, and the job's text is `"日本"`; overlapping `[(0..4, String), (1..2, Number)]` on `"abcd"` gives a Number byte 1; `CodeTheme::from_visuals(&Visuals::dark()) != CodeTheme::from_visuals(&Visuals::light())`; `set_color` then `color` round-trips. Editor tests in `tests/editor.rs` with a test highlighter that counts its calls (`Cell<usize>`) and marks every `fn` as `Keyword` for language `"test"`, `None` otherwise: `a_highlighter_colours_the_text` (a section with the keyword colour), `an_unknown_language_is_plain` (job sections equal to an editor with no highlighter), `the_highlighter_runs_once_for_unchanged_text` (10 frames → 1 call; one typed char → 2).
- [ ] **Step 2:** Run `cargo test -p alumina-code`: fails to compile.
- [ ] **Step 3:** Implement as above; crate docs and README gain a highlighting section (the trait, a 10-line example, the colours follow the theme, a swapped highlighter takes effect at the next change); CHANGELOG Added line `*(alumina-code)* syntax highlighting through the app's Highlighter, coloured by CodeTheme`.
- [ ] **Step 4:** `cargo test -p alumina-code --all-targets --features serde`, clippy, fmt, `cargo doc -p alumina-code --no-deps`: pass.
- [ ] **Step 5:** Commit `feat(alumina-code): highlighting through the app's Highlighter (#15)`.

---

### Task 2: alumina-markdown — front matter and code block content in the parser

**Files:**
- Modify: `crates/alumina-markdown/src/parse.rs`

**Interfaces (produces):**
- `BlockKind::FrontMatter` (unit variant).
- `Block::content: Vec<Range<usize>>` — for `CodeBlock` and `FrontMatter`, the byte ranges of the code in source order, one per text piece pulldown-cmark gives (so without fences, and in a list or quote without the line prefixes); empty for other blocks.

Parse with `Options::ENABLE_YAML_STYLE_METADATA_BLOCKS` added. A `Tag::MetadataBlock(MetadataBlockKind::YamlStyle)` becomes a `FrontMatter` block over the event's range, with marks the first line's `---` and the closing line's `---` or `...` (without their line breaks); its content is the text events inside.

- [ ] **Step 1: Failing tests** in `parse.rs`:
  - `front_matter_at_the_start`: `"---\ntitle: a\n---\n\n# h\n"` → blocks[0] is `FrontMatter`, range `0..17`, marks `[0..3, 13..16]`, content `[4..13]`; a `Heading` follows.
  - `front_matter_closed_by_dots`: `"---\na: 1\n...\n"` → `FrontMatter` with marks `[0..3, 9..12]`.
  - `dashes_elsewhere_are_not_front_matter`: `"a\n\n---\nb: 1\n---\n"` → no `FrontMatter`; a `ThematicBreak` is there.
  - `an_unclosed_front_matter_while_typing`: `"---\n"` and `"---\ntitle: a\n"` → no `FrontMatter`, no panic (whatever pulldown-cmark makes of them otherwise, e.g. a thematic break); and `"---\ntitle: a\n\n# h"` still has the heading.
  - `code_block_content`: ```` "```rust\nfn a() {}\n```\n" ```` → content `[8..18]`; in a quote ```` "> ```rust\n> let a;\n> ```\n" ```` → each content range's text is `"let a;\n"` joined across pieces (assert the concatenation of `&source[r]` equals `"let a;\n"`), and no range contains `>`; in a list item ```` "- a\n\n  ```yaml\n  k: v\n  ```\n" ```` → concatenation `"k: v\n"`.
  - Existing tests still pass.
- [ ] **Step 2:** `cargo test -p alumina-markdown --lib`: fails.
- [ ] **Step 3:** Implement. Check pulldown-cmark 0.13.6's event ranges for metadata blocks and adjust the expected values only if the source of truth differs, saying so in the report.
- [ ] **Step 4:** `cargo test -p alumina-markdown`, clippy, fmt: pass.
- [ ] **Step 5:** Commit `feat(alumina-markdown): YAML front matter, and each code block's content (#14)`.

---

### Task 3: alumina-markdown — look and highlighting

**Files:**
- Modify: `crates/alumina-markdown/Cargo.toml` (`alumina-code = { workspace = true }`), `src/style.rs`, `src/layout.rs`, `src/editor.rs`, `src/lib.rs`, `tests/editor.rs`, `tests/speed.rs`, `README.md`, `CHANGELOG.md`

**Interfaces:**
- Consumes: `Highlighter`, `TokenKind`, `CodeTheme` (Task 1); `BlockKind::FrontMatter`, `Block::content` (Task 2).
- Produces: `MarkdownStyle::code_theme: CodeTheme` (from the same visuals; part of its `Hash`); `MarkdownEditor::highlighter(&'t dyn Highlighter)`; re-export `alumina_code::{Highlighter, TokenKind, CodeTheme}` from alumina-markdown's lib.

- `layout` (public, unchanged signature) gives `FrontMatter` the code block's look (monospace size, background) and its marks the mark colour, as it does for fences.
- Highlighting, inside the editor's layout: for each `CodeBlock { info: Some(info) }` (language = the first whitespace-separated word of `info`) and each `FrontMatter` (language `"yaml"`), join the `content` pieces into one string, call the highlighter, map each returned range from the joined string back to source bytes (a range crossing two pieces is split), sanitise per Global Constraints, and set the colour of those bytes to `style.code_theme.color(kind)` before the sections are built (marks keep the mark colour). Indented code blocks and fences without info are left alone.
- Caching: the editor's `FrameCache` stays for the no-highlighter case; with a highlighter the rendered value is kept in `ui.data` under `id.with("highlight")` keyed by `hash((text, style, wrap_width bits))`, as in Task 1, so the highlighter runs only on change.

- [ ] **Step 1: Failing tests** in `tests/editor.rs`, with a test highlighter (counts calls; for `"rust"` marks `fn` as `Keyword`, for `"yaml"` marks each `key` before `:` as `Key`; records the languages it was asked): `front_matter_is_highlighted_as_yaml` (`"---\ntitle: a\n---\n"`: asked `"yaml"`, the bytes of `title` have the theme's Key colour, the `---` the mark colour); `a_code_block_in_a_quote_lands_on_its_code` (```` "> ```rust\n> fn a\n> ```\n" ````: the `fn` bytes have the Keyword colour and the `>` bytes do not); `the_language_is_the_first_word_of_the_info` (```` "```rust ignore\nfn\n```" ```` asks `"rust"`); `an_indented_block_is_not_highlighted` (not asked); `bad_ranges_do_not_panic` (a highlighter returning `[(0..999, Keyword), (5..2, Key)]`); `the_highlighter_runs_once_for_unchanged_text`; `no_highlighter_looks_as_before` (sections equal with and without `.highlighter` of a highlighter returning `None` for everything). In `tests/speed.rs`, a variant of the frame timing with a cheap highlighter (every ASCII word in a `rust` block a `Keyword`), reported like the others.
- [ ] **Step 2:** `cargo test -p alumina-markdown --test editor`: fails.
- [ ] **Step 3:** Implement; README gains a short highlighting and front matter section; CHANGELOG Added lines `*(alumina-markdown)* YAML front matter, shown as code` and `*(alumina-markdown)* code blocks and front matter coloured by the app's Highlighter`.
- [ ] **Step 4:** `cargo test -p alumina-markdown --all-targets`, `cargo test -p alumina-markdown --release --test speed -- --ignored --nocapture` (report the numbers), clippy, fmt, doc: pass.
- [ ] **Step 5:** Commit `feat(alumina-markdown): highlighted code blocks and front matter (#15, #14)`.

---

### Task 4: the demo's highlighters

**Files:**
- Create: `demo/src/highlight.rs` (`DemoHighlighter`), `demo/src/highlight/yaml.rs`, `demo/src/highlight/rust.rs`
- Modify: `demo/Cargo.toml` (`syntect = { version = "5", default-features = false, features = ["default-syntaxes", "regex-fancy"] }`), `demo/src/lib.rs` (`mod highlight;`), `deny.toml` (an advisory ignore for bincode's unmaintained notice if cargo-deny flags it, with its reason: reached only by the demo through syntect's syntax dumps)

**Interfaces:**
- `pub struct DemoHighlighter` (Default) implementing `alumina_code::Highlighter`: `"yaml" | "yml"` → `yaml::highlight(code)`, `"rust" | "rs"` → `rust::highlight(code)`, else `None`; language matched case-insensitively.
- `yaml::highlight(code: &str) -> Vec<(Range<usize>, TokenKind)>`, `rust::highlight(code: &str) -> Vec<(Range<usize>, TokenKind)>`.

YAML by hand, line by line: a comment (`#` at the line's start or after whitespace, not inside quotes) → `Comment`; `---` / `...` alone on a line → `Punctuation`; a leading `- ` → its `-` `Punctuation`; a key (`key:` followed by space or end) → `Key`, its `:` `Punctuation`; a value: `"…"` / `'…'` → `String`; an integer or float → `Number`; `true` `false` `null` `~` (and `yes`/`no` not) → `Constant`; `&name` `*name` `!tag` → `Attribute`; any other plain scalar → `String`; flow `[ ] { } ,` → `Punctuation`.

Rust over syntect: a `SyntaxSet::load_defaults_newlines()` loaded once (`std::sync::LazyLock`), the Rust syntax by extension `"rs"`, `ParseState` per call, each line's `ScopeStackOp`s applied to a `ScopeStack`; each run of text gets the kind of the innermost scope that maps (prefix match on the scope's string, first rule that matches): `comment` → Comment; `string` → String; `constant.numeric` → Number; `constant.language`, `constant.character` → Constant; `entity.name.function`, `support.function`, `support.macro`, `entity.name.macro` → Function; `entity.name.type`, `support.type`, `storage.type` except `storage.type.function` → Type; `keyword`, `storage` → Keyword; `meta.attribute`, `meta.annotation` → Attribute; `punctuation` → Punctuation. Unmapped runs get no token.

- [ ] **Step 1: Failing tests** in each module. YAML on
  ```
  # c
  key: value # note
  n: 12
  b: true
  s: "q # not"
  - item
  a: &anc x
  r: *anc
  t: !tag y
  ---
  ```
  expects (as `&code[range]`, kind) at least: `("# c", Comment)`, `("key", Key)`, `(":", Punctuation)`, `("value", String)`, `("# note", Comment)`, `("12", Number)`, `("true", Constant)`, `("\"q # not\"", String)` and no Comment inside it, `("-", Punctuation)`, `("&anc", Attribute)`, `("*anc", Attribute)`, `("!tag", Attribute)`, `("---", Punctuation)`; and non-ASCII `"名前: 太郎"` → `("名前", Key)`, `("太郎", String)`. Rust on `"fn main() {\n    let s = \"x\"; // c\n    println!(\"{}\", 1);\n}\n"` expects a token of each: Keyword covering `fn` or `let`, Function covering `main`, String covering `"x"`, Comment covering `// c`, Number covering `1`. `DemoHighlighter`: `"YML"` answers, `"json"` is `None`.
- [ ] **Step 2:** `cargo test -p alumina-demo highlight`: fails.
- [ ] **Step 3:** Implement. Run `cargo deny check` if installed (else note it is CI's) and add the ignore only for an advisory actually reported, with its id.
- [ ] **Step 4:** `cargo test -p alumina-demo`, clippy, fmt, `cargo build -p alumina-demo --bin alumina-demo`: pass. `cargo tree -p alumina-code -e normal` and `-p alumina-markdown` show no syntect.
- [ ] **Step 5:** Commit `feat(demo): a YAML highlighter by hand and a Rust one over syntect (#15)`.

---

### Task 5: the demo's pages and the docs

**Files:**
- Create: `demo/src/code-sample.yaml`
- Modify: `demo/src/code.rs` (a YAML sample, default unit 2 spaces; `.language(..)` per sample — `"json"`, `"yaml"`, `"rust"` — and `.highlighter(&self.highlighter)` with a `DemoHighlighter` field), `demo/src/markdown.rs` (`.highlighter(..)`), `demo/src/markdown-sample.md` (starts with a YAML front matter of 3–5 keys including a list; gains a ```` ```rust ```` block and a ```` ```yaml ```` block inside a list item), `demo/README.md` (Code and Markdown rows mention the highlighting and front matter, and that the highlighters are the demo's own: YAML by hand, Rust over syntect), `README.md` (alumina-code's row mentions highlighting through the app's highlighter)

- [ ] **Step 1:** Write the sample and wire both pages.
- [ ] **Step 2:** `cargo build -p alumina-demo --bin alumina-demo`, `cargo test --workspace`, clippy, fmt, `cargo publish --workspace --dry-run --locked`: pass.
- [ ] **Step 3:** Commit `feat(demo): highlighted Code and Markdown pages, with front matter (#15, #14)`.
