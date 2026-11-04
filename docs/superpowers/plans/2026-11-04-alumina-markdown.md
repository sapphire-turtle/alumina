# alumina-markdown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An egui Markdown editor that shows the source as it is, formatted, with
shortcuts, a toolbar and list completion, and the same look read-only.

**Architecture:** `parse` turns the source into a `Doc` of blocks and inlines with byte
ranges (pulldown-cmark's offset iterator, marks found at the elements' edges); `layout`
turns a `Doc` into an egui `LayoutJob`, which a `TextEdit`'s layouter returns, so egui keeps
the cursor, the IME and undo. `commands` are pure edits on a string and a byte selection;
the editor's keys and the toolbar call them.

**Tech Stack:** Rust 2024, egui 0.37, pulldown-cmark 0.13, egui_kittest 0.37 (tests).

**Spec:** `docs/superpowers/specs/2026-11-04-alumina-markdown-design.md`

## Global Constraints

- Dependencies of `alumina-markdown`: `egui` (workspace) and `pulldown-cmark = { version =
  "0.13", default-features = false }`. Nothing from corundum.
- Every range in `Doc` and `commands` is a byte range of the source on char boundaries.
  egui's cursors count chars: convert at the editor's edge, nowhere else.
- Marks are never hidden; they are drawn in `MarkdownStyle::mark_color`.
- No ticking of `[ ]` by clicking, read-only or editing.
- One pull request per issue, in the order #3, #4, #5, #6, each with `Closes #N`; the #6
  PR also closes #1. Branch `feat/markdown` for #3, then `feat/markdown-editor`,
  `feat/markdown-commands`, `feat/markdown-read-only`, each from `main` after the last
  merge.
- Before each commit: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace`.
- Doc comments and prose in English, in the style of alumina-datetime (short, plain).

## Review Focus

1. **Japanese and emoji around a selection.** A toolbar click or shortcut on text like
   `日本語🎌` must wrap exactly the selected chars; a byte/char mix-up panics or cuts a
   char. Pinned in Task 7 and Task 10.
2. **Enter while composing with an IME.** Enter that commits a conversion must not also
   continue a list. Pinned in Task 10.
3. **Half-typed syntax.** Every prefix of the sample document (`**bold` with no close, an
   open fence) parses without panicking, ranges in bounds and on char boundaries. Pinned in
   Task 2.
4. **CRLF sources.** A file saved on Windows (`\r\n`) gets the same blocks and marks, and
   list continuation inserts `\r\n`. Pinned in Task 1 and Task 9.
5. **The ends of the text.** Commands with the cursor at 0, at `len`, on an empty source,
   and on a last line with no newline. Pinned in Task 8 and Task 9.

---

## PR 1 (#3): the parser

### Task 1: Doc types and block parsing

**Files:**
- Modify: `crates/alumina-markdown/Cargo.toml`, `crates/alumina-markdown/src/lib.rs`
- Create: `crates/alumina-markdown/src/parse.rs` (tests in a `#[cfg(test)] mod tests` at
  its end, as alumina-datetime does)

**Interfaces:**
- Produces:
  ```rust
  pub fn parse(source: &str) -> Doc;
  #[derive(Clone, Debug, Default, PartialEq)]
  pub struct Doc { pub blocks: Vec<Block>, pub inlines: Vec<Inline> }
  #[derive(Clone, Debug, PartialEq)]
  pub struct Block { pub kind: BlockKind, pub range: Range<usize>, pub marks: Vec<Range<usize>> }
  #[derive(Clone, Debug, PartialEq)]
  pub enum BlockKind {
      Paragraph,
      Heading { level: u8 },                       // 1..=6
      ListItem { depth: usize, number: Option<u64>, task: Option<bool> }, // depth 0 = top
      Quote { depth: usize },                      // 1 = `>`
      CodeBlock { info: Option<String> },          // None for indented or a bare fence
      ThematicBreak,
  }
  ```
  `Inline`/`InlineKind` are declared here as in Task 2; `Doc::inlines` stays empty until
  then. Re-export `parse`, `Doc`, `Block`, `BlockKind`, `Inline`, `InlineKind` and `egui`
  from `lib.rs`.

- [ ] **Step 1: Write the failing tests** in `parse.rs`, with a helper
  `fn marks(src: &str, b: &Block) -> Vec<&str>` giving the marks as text:
  - `atx_heading`: `"## Title\n"` → one block `Heading { level: 2 }`, range `0..9`, marks
    `["##"]`.
  - `setext_heading`: `"Title\n===\n"` → `Heading { level: 1 }`, marks `["==="]`.
  - `nested_lists`: `"- a\n  1. b\n- [x] c\n"` → `ListItem { depth: 0, number: None, task:
    None }` marks `["-"]`; `ListItem { depth: 1, number: Some(1), task: None }` marks
    `["1."]`; `ListItem { depth: 0, number: None, task: Some(true) }` marks `["-", "[x]"]`.
  - `quote_nested`: `"> a\n> > b\n"` → `Quote { depth: 1 }` marks `[">", ">"]` (one per
    line) and `Quote { depth: 2 }` marks `[">"]` (the inner one, line 2).
  - `fenced_code`: `` "```rust\nlet a = 1;\n```\n" `` → `CodeBlock { info:
    Some("rust".into()) }`, marks `["```rust", "```"]`; no `Paragraph` inside it.
  - `thematic_break`: `"a\n\n---\n"` → a `Paragraph` and a `ThematicBreak` with marks
    `["---"]`.
  - `japanese_ranges`: `"# 見出し🎌\n- 項目\n"` → heading range `0..16`, every range
    satisfies `src.is_char_boundary`.
  - `crlf`: the `nested_lists` source with `\n` replaced by `\r\n` gives the same kinds and
    the same mark texts.

- [ ] **Step 2: Run to see them fail**

  Run: `cargo test -p alumina-markdown parse`
  Expected: FAIL (`parse` not defined).

- [ ] **Step 3: Implement `parse` (blocks)**

  Add `pulldown-cmark` (Global Constraints) and `egui = { workspace = true }`. Parse with
  `Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS` and `into_offset_iter`. Keep a
  stack of open list and quote containers for `depth`; `number` is the list's start plus the
  item's index. Marks, from the source at the element's range: a heading's leading `#` run
  or its setext underline line; a list item's bullet/number at the start of its first line
  after indentation, and the `TaskListMarker` event's range; a quote's `>` at the start of
  each of its lines after the outer quotes' `>`; a fence's opening line (trimmed of the line
  ending) and closing line. A block's `range` is pulldown-cmark's: marks and the trailing
  line ending included.

- [ ] **Step 4: Run to see them pass**

  Run: `cargo test -p alumina-markdown parse`
  Expected: PASS.

- [ ] **Step 5: Commit** — `feat(alumina-markdown): parse blocks with their marks (#3)`

### Task 2: inline parsing

**Files:**
- Modify: `crates/alumina-markdown/src/parse.rs`

**Interfaces:**
- Consumes: Task 1's `parse`, `Doc`.
- Produces:
  ```rust
  #[derive(Clone, Debug, PartialEq)]
  pub struct Inline { pub kind: InlineKind, pub range: Range<usize>, pub marks: Vec<Range<usize>> }
  #[derive(Clone, Debug, PartialEq)]
  pub enum InlineKind { Emphasis, Strong, Strikethrough, Code,
      Link { url: String, text: Range<usize> } }
  ```
  `Doc::inlines` in source order; an inline inside another comes after it.

- [ ] **Step 1: Write the failing tests**
  - `strong_and_emphasis`: `"**a** *b* _c_ __d__"` → `Strong` marks `["**", "**"]`,
    `Emphasis` marks `["*", "*"]`, `Emphasis` `["_", "_"]`, `Strong` `["__", "__"]`.
  - `nested_emphasis`: `"***a***"` → a `Strong` and an `Emphasis`, all marks inside `0..7`,
    together covering the six `*`.
  - `strikethrough`: `"~~a~~"` → marks `["~~", "~~"]`.
  - `inline_code`: `` "`` a ` b ``" `` → `Code` marks `` ["``", "``"] ``.
  - `link`: `"[文字](https://e.x)"` → `Link { url: "https://e.x", text }` with
    `&src[text] == "文字"`, marks `["[", "](https://e.x)"]`.
  - `autolink`: `"<https://e.x>"` → `Link` with marks `["<", ">"]`.
  - `nothing_inside_code_blocks`: `` "```\n**a**\n```\n" `` → `inlines` empty.
  - `every_prefix_parses`: for each char boundary `i` of a sample containing every piece of
    syntax (keep it as `const SAMPLE: &str` in `parse.rs`, `#[cfg(test)]`; Task 6 copies it
    into the demo), `parse(&SAMPLE[..i])` does not panic and every block, inline and mark
    range is within `0..=i` and on char boundaries.

- [ ] **Step 2: Run to see them fail** — `cargo test -p alumina-markdown parse`; the new
  tests FAIL.

- [ ] **Step 3: Implement the inlines.** From `Start(Emphasis|Strong|Strikethrough|Link)`
  and `Code`'s ranges; delimiters are the run of the same char at each end of the range
  (as long as the kind needs: 1, 2, 2), the code's backtick run at each end; a link's
  marks are `[` and the part from the `]` after the text to the range's end, its `text` the
  range of its `Text` events. An autolink's marks are its `<` and `>`.

- [ ] **Step 4: Run to see them pass** — `cargo test -p alumina-markdown` PASS.

- [ ] **Step 5: Commit** — `feat(alumina-markdown): parse inlines with their delimiters (#3)`

### Task 3: measure, and the #3 pull request

**Files:**
- Create: `crates/alumina-markdown/tests/speed.rs`

- [ ] **Step 1: Write `parse_ten_thousand_lines`** (`#[ignore]`): `SAMPLE` repeated to
  10,000 lines (build it in the test from a copy of the sample's text), parse it 20 times,
  `eprintln!` the mean time per parse. No assertion.

- [ ] **Step 2: Run it** — `cargo test -p alumina-markdown --release --test speed --
  --ignored --nocapture`. Note the mean. Under 8 ms (half a 60 Hz frame) keeps the whole
  reparse; above, add a task before PR 2 that reparses only from the first changed block
  (and say so in the PR).

- [ ] **Step 3: Commit, push, open the PR** — commit `test(alumina-markdown): time a
  10,000-line parse (#3)`; PR "alumina-markdown: the parser" with the spec and this plan,
  the measured time, `Closes #3`. Wait for CI green; merge with a merge commit.

## PR 2 (#4): the formatted editor

### Task 4: style and layout

**Files:**
- Create: `crates/alumina-markdown/src/style.rs`, `crates/alumina-markdown/src/layout.rs`

**Interfaces:**
- Consumes: `Doc`, `Block`, `Inline`.
- Produces:
  ```rust
  #[derive(Clone, Debug, PartialEq)]
  pub struct MarkdownStyle {
      pub body: egui::FontId, pub monospace: egui::FontId,
      pub heading_sizes: [f32; 6],
      pub text_color: Color32, pub strong_color: Color32, pub mark_color: Color32,
      pub link_color: Color32,
      pub quote_color: Color32, pub code_background: Color32,
  }
  impl MarkdownStyle { pub fn from_visuals(visuals: &egui::Visuals) -> Self }
  impl std::hash::Hash for MarkdownStyle  // for the layout cache (hash the f32s by bits)
  pub fn layout(source: &str, doc: &Doc, style: &MarkdownStyle, wrap_width: f32) -> egui::text::LayoutJob;
  ```
  Defaults: body 14 proportional, monospace 13, `heading_sizes = [24.0, 20.0, 18.0, 16.0,
  14.0, 14.0]`, `text_color = visuals.text_color()`, `strong_color = visuals.strong_text_color()`,
  `mark_color = visuals.weak_text_color()`,
  `link_color = visuals.hyperlink_color`, `quote_color = visuals.weak_text_color()`,
  `code_background = visuals.code_bg_color`.

- [ ] **Step 1: Write the failing tests** in `layout.rs`, with a helper `fn at(job:
  &LayoutJob, byte: usize) -> &TextFormat` (the section whose `byte_range` holds it):
  - `job_text_is_the_source`: `job.text == source` for `SAMPLE`.
  - `heading_size`: in `"## T\n"`, byte 3's `font_id.size == 20.0`.
  - `marks_are_faint`: in `"**a**"`, bytes 0 and 3 have `mark_color`, byte 2 has
    `text_color`.
  - `strong_and_emphasis_and_strike`: strong text has `strong_color` (egui ships no bold
    face, so strong is the brighter colour, as egui's own `RichText::strong`); emphasis has
    `italics == true`; strikethrough has a non-`NONE` `strikethrough` stroke.
  - `code`: inline and block code use `monospace` and `background == code_background`;
    inside a fenced block `**a**` has no mark colour.
  - `link`: the link text has `link_color` and an underline.
  - `wrap_width`: `job.wrap.max_width == wrap_width`.

- [ ] **Step 2: Run** — FAIL.

- [ ] **Step 3: Implement.** Compute a format per byte from the blocks (outer to inner)
  then the inlines, then the marks; append a section per run of equal formats.

- [ ] **Step 4: Run** — PASS.

- [ ] **Step 5: Commit** — `feat(alumina-markdown): lay out a Doc with a format per range (#4)`

### Task 5: `MarkdownEditor`

**Files:**
- Create: `crates/alumina-markdown/src/editor.rs`, `crates/alumina-markdown/tests/editor.rs`
- Modify: `crates/alumina-markdown/Cargo.toml` (dev-dependencies as alumina-datetime's)

**Interfaces:**
- Consumes: `parse`, `layout`, `MarkdownStyle`.
- Produces:
  ```rust
  pub struct MarkdownEditor<'t> { /* text, id, read_only, style, desired_rows */ }
  impl<'t> MarkdownEditor<'t> {
      pub fn new(text: &'t mut String) -> Self;
      pub fn id(self, id: egui::Id) -> Self;       // default: Id::new("alumina-markdown")
      pub fn read_only(self, read_only: bool) -> Self;
      pub fn style(self, style: MarkdownStyle) -> Self; // default from_visuals at show
      pub fn desired_rows(self, rows: usize) -> Self;
      pub fn show(self, ui: &mut egui::Ui) -> MarkdownOutput;
  }
  pub struct MarkdownOutput { pub changed: bool, pub link_clicked: Option<String>,
      pub text_edit: egui::text_edit::TextEditOutput }
  pub(crate) fn char_to_byte(s: &str, c: usize) -> usize;
  pub(crate) fn byte_to_char(s: &str, b: usize) -> usize;
  ```
  The spec writes `.id_salt("note")`; the plan uses `.id(Id)` instead, since the toolbar
  (Task 11) needs the same id before the editor shows.

- [ ] **Step 1: Write the failing tests** in `tests/editor.rs` (a `render` helper as in
  alumina-datetime's tests):
  - `types_and_reports_a_change`: focus the text input (`get_by_role(Role::MultilineTextInput)`
    → `click()`), `type_text("# hi")`, run → state text `"# hi"`, an output with `changed`
    seen.
  - `read_only_does_not_change`: same with `.read_only(true)` → text unchanged, `changed`
    never true.
  - `ime_composes_and_commits`: focus, send `Event::Ime(ImeEvent::Enabled)`,
    `Preedit("にほ")`, run, then `Commit("日本")`, run → text `"日本"`, no panic during the
    preedit frame.
  - `layout_follows_the_text`: after typing `"**a**"`, `text_edit.galley.job.sections`
    has more than one section.

- [ ] **Step 2: Run** — FAIL.

- [ ] **Step 3: Implement.** Editing: `TextEdit::multiline(text)`; read-only:
  `TextEdit::multiline(&mut text.as_str())`, which egui selects and copies but does not
  edit. Both `.id(id).desired_rows(..).desired_width(f32::INFINITY).layouter(..)`. The
  layouter parses and lays out through `egui::util::cache::FrameCache` keyed by the text, the
  style and the wrap width, then `ui.fonts_mut(|f| f.layout_job(job))`. After the
  `TextEdit`, for each `ThematicBreak` paint a `visuals.widgets.noninteractive.bg_stroke`
  line across `text_edit.text_clip_rect` at the vertical centre of its row in the galley.

- [ ] **Step 4: Run** — `cargo test -p alumina-markdown` PASS.

- [ ] **Step 5: Commit** — `feat(alumina-markdown): MarkdownEditor, a TextEdit laid out as Markdown (#4)`

### Task 6: the demo page, and the #4 pull request

**Files:**
- Create: `demo/src/markdown.rs`
- Modify: `demo/Cargo.toml` (`alumina-markdown = { workspace = true }`), `demo/src/lib.rs`
  (the `Page::Markdown` arm)

- [ ] **Step 1: Write `MarkdownDemo`**: `text` (the sample from Task 2, copied as a
  `const`), `read_only: bool`, `last_link: Option<String>`; `ui()` shows a "Read-only"
  checkbox, the editor in a `ScrollArea::vertical`, and the last link clicked. Replace the
  "Not yet" label with it.

- [ ] **Step 2: Run the demo** — `cargo run -p alumina-demo`; on a machine with a display,
  check the look in dark and light, and type Japanese with the system IME (record what was
  seen in the PR).

- [ ] **Step 3: Commit, push, PR** — `feat(alumina-demo): a page for alumina-markdown's
  editor (#4)`; PR "alumina-markdown: the formatted editor", a render of the editor from
  `ALUMINA_RENDER_DIR`, `Closes #4`. CI green; merge.

## PR 3 (#5): help with the marks

### Task 7: inline commands

**Files:**
- Create: `crates/alumina-markdown/src/commands.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, PartialEq, Eq)]
  pub struct Edit { pub replace: Range<usize>, pub with: String, pub selection: Range<usize> }
  impl Edit { pub fn apply(&self, text: &mut String) } // selection is in the edited text
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum Command { Strong, Emphasis, Strikethrough, Code, Link,
      Heading(u8), BulletList, NumberedList, TaskList, Quote, CodeBlock }
  pub fn command(source: &str, selection: Range<usize>, command: Command) -> Option<Edit>;
  ```
  `Heading(0)` removes a heading. `None` when there is nothing to do.

- [ ] **Step 1: Write the failing tests** (a helper `run(src, sel, cmd) -> (String,
  Range<usize>)`):
  - `strong_wraps_selection`: `("a b c", 2..3, Strong)` → `("a **b** c", 4..5)`.
  - `strong_unwraps`: `("a **b** c", 4..5, Strong)` → `("a b c", 2..3)`; also with the
    selection over the marks, `2..7`.
  - `word_at_cursor`: `("hello world", 8..8, Emphasis)` → `("hello *world*", 9..9)`.
  - `empty_cursor_inserts_pair`: `("a  b", 2..2, Code)` → `` ("a `` b", 3..3) ``.
  - `link`: `("see this", 4..8, Link)` → `("see [this](url)", 11..14)`.
  - `japanese_and_emoji`: `("日本語🎌です", 3..13, Strong)` (bytes of `本語🎌`) → `("日**本語🎌**です", 5..15)`.

- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** the inline arm of `command`. The word at the cursor is the run of
  `char::is_alphanumeric` chars around it (kana and kanji count, so a Japanese run between
  punctuation is one word); with none, insert the empty pair. Unwrap when the delimiters
  sit just outside or at the edges of the selection.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** — `feat(alumina-markdown): toggle inline marks on a selection (#5)`

### Task 8: line commands

**Files:** Modify `crates/alumina-markdown/src/commands.rs`

- [ ] **Step 1: Write the failing tests**
  - `heading_levels`: `("a", 0..0, Heading(2))` → `"## a"`; `("# a", 0..0, Heading(2))` →
    `"## a"`; `("## a", 0..0, Heading(2))` → `"a"` (same level toggles off);
    `("## a", 0..0, Heading(0))` → `"a"`.
  - `lists_over_lines`: `("a\nb", 0..3, BulletList)` → `"- a\n- b"`; again → `"a\nb"`;
    `NumberedList` → `"1. a\n2. b"`; `TaskList` → `"- [ ] a\n- [ ] b"`; `BulletList` on
    `"1. a"` → `"- a"`.
  - `quote`: `("a\nb", 0..3, Quote)` → `"> a\n> b"`, and back.
  - `code_block`: `("a\nb", 0..3, CodeBlock)` → `` "```\na\nb\n```" ``, and back.
  - `ends`: each line command on `""` with `0..0`, and on `"a\nb"` with `3..3` (the end, no
    newline) gives a valid edit and a selection within the new text.
  - The selection after each keeps covering the same text (shifted by the marks).
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** the line arms: the lines the selection touches; toggle off when
  every touched line already has the mark.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** — `feat(alumina-markdown): toggle line marks: headings, lists, quotes, code (#5)`

### Task 9: list continuation and indentation

**Files:** Modify `crates/alumina-markdown/src/commands.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn newline(source: &str, cursor: usize) -> Option<Edit>; // None: not in a list item
  pub fn indent(source: &str, selection: Range<usize>, outdent: bool) -> Option<Edit>;
  ```
- [ ] **Step 1: Write the failing tests**
  - `continues`: `newline("- a", 3)` → `"- a\n- "`, cursor 6; `"3. a"` → `"3. a\n4. "`;
    `"- [x] a"` → `"- [x] a\n- [ ] "`; `"  - a"` keeps the indent.
  - `empty_item_leaves`: `newline("- a\n- ", 6)` → `"- a\n"`, cursor 4.
  - `not_in_list`: `newline("a", 1)` → `None`; in a code block → `None`.
  - `mid_item_splits`: `newline("- ab", 3)` → `"- a\n- b"`, cursor 6.
  - `crlf`: `newline("- a\r\n- b", 3)` inserts `"\r\n- "`.
  - `indent`: `indent("- a\n- b", 0..7, false)` → `"  - a\n  - b"`; `outdent` back; a
    numbered item indents by its marker's width (`"1. a"` → `"   1. a"`).
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** with `parse` to know whether the cursor's line is a list item.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** — `feat(alumina-markdown): continue, leave and indent lists (#5)`

### Task 10: the editor's keys

**Files:** Modify `crates/alumina-markdown/src/editor.rs`, `crates/alumina-markdown/tests/editor.rs`

- [ ] **Step 1: Write the failing tests** (kittest, editing, focused):
  - `ctrl_b_wraps`: text `"a b"`, select `2..3` through `TextEditState` (store a
    `CCursorRange` before the frame), press `Modifiers::COMMAND` + `Key::B` → `"a **b**"`.
  - Likewise `+I` Emphasis, `+K` Link, `+Shift+X` Strikethrough, `+E` Code (one test,
    table-driven).
  - `enter_continues_list`: text `"- a"`, cursor at end, `Key::Enter` → `"- a\n- "`.
  - `tab_indents`: text `"- a"`, `Key::Tab` → `"  - a"`; `Shift+Tab` back. Outside a list,
    Tab inserts what egui inserts (unchanged behaviour).
  - `japanese_selection_shortcut`: `"日本語"`, chars `1..2` selected, `COMMAND+B` →
    `"日**本**語"`, selection chars `3..4`.
  - `enter_during_ime_does_not_continue`: text `"- a"`, cursor at end, `Ime(Preedit("か"))`
    then in the same frame `Key::Enter` and `Ime(Commit("か"))` → `"- aか"`, no new item.
  - `read_only_ignores_keys`: read-only, `COMMAND+B` → unchanged.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement.** When the `TextEdit` has focus and is editable, before it shows:
  read the selection from `TextEditState::load`, convert chars to bytes, and
  `consume_key` the shortcuts; Enter/Tab/Shift+Tab are consumed only when `newline`/`indent`
  return `Some`, and Enter not at all in a frame with an `Ime::Preedit` or `Ime::Commit`
  event, or while the editor's own memory says a preedit is open (set by `Preedit` with
  text, cleared by `Commit` and `Disabled`). Apply the `Edit`, store the new selection as
  chars.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** — `feat(alumina-markdown): the editor's shortcuts and list keys (#5)`

### Task 11: `MarkdownToolbar`, and the #5 pull request

**Files:**
- Create: `crates/alumina-markdown/src/toolbar.rs`
- Modify: `crates/alumina-markdown/tests/editor.rs`, `demo/src/markdown.rs`,
  `crates/alumina-markdown/Cargo.toml` (dev-dependency `egui_extras = { version = "0.37",
  features = ["svg"] }`, for the test's loader only)

**Interfaces:**
- Produces:
  ```rust
  pub struct MarkdownToolbar<'t> { /* text, editor_id, labels */ }
  impl<'t> MarkdownToolbar<'t> {
      pub fn new(text: &'t mut String, editor_id: egui::Id) -> Self;
      pub fn labels(self, labels: ToolbarLabels) -> Self;
      pub fn icons(self, icons: ToolbarIcons) -> Self;
      pub fn show(self, ui: &mut egui::Ui) -> egui::Response; // changed() when it edited
  }
  #[derive(Clone, Debug)]
  pub struct ToolbarLabels { pub strong: String, pub emphasis: String, pub strikethrough: String,
      pub code: String, pub link: String, pub heading: String, pub bullet_list: String,
      pub numbered_list: String, pub task_list: String, pub quote: String, pub code_block: String }
  #[derive(Clone, Debug, Default)]
  pub struct ToolbarIcons { /* the same eleven fields */ pub strong: Option<egui::ImageSource<'static>>, .. }
  ```
  Label defaults, ASCII only (egui's default fonts may lack `☐`, `❝`): `"B"`, `"I"`, `"S"`,
  `"</>"`, `"Link"`, `"H"` (cycles 1→2→3→none), `"-"`, `"1."`, `"[ ]"`, `">"`, `"{ }"`.
  A button with an icon shows the icon alone, sized to `ui.spacing().interact_size.y`
  less the button padding, tinted `visuals.text_color()`; its label then becomes its
  accessible name. A button without one shows the label. Either way the hover text is the
  label and, for the commands with one, the shortcut (`"B (Ctrl+B)"`, `⌘` on macOS via
  `ctx.format_shortcut`); an app that wants words there passes words as labels. alumina ships no icons and no image loader: the app passes
  `ImageSource`s and installs the loader they need (an SVG one for Lucide).
- [ ] **Step 1: Write the failing tests**: a harness with the toolbar above the editor:
  select `2..3` in `"a b"`, click the `"B"` button → `"a **b**"`, the editor's selection is
  `4..5` and it has focus again; a custom label `"太字"` is the button found by label; with
  `ToolbarIcons { strong: Some(ImageSource::Bytes { uri: "bytes://b.svg".into(), bytes:
  SVG.into() }), .. }` (a tiny SVG in the test, `egui_extras` loaders installed in the
  harness) the strong button is still found by the label `"B"` and still toggles.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement** with `command`, the same selection loading and storing as
  Task 10 (factor them into `editor.rs` `pub(crate) fn selection(ctx, id, text) ->
  Range<usize>` and `set_selection`), and `memory.request_focus(editor_id)`.
- [ ] **Step 4: Run** — PASS. Add the toolbar to the demo page.
- [ ] **Step 5: Commit, push, PR** — `feat(alumina-markdown): MarkdownToolbar (#5)`; PR
  "alumina-markdown: shortcuts, toolbar, list completion", `Closes #5`. CI green; merge.

## PR 4 (#6): reading

### Task 12: link clicks

**Files:** Modify `crates/alumina-markdown/src/editor.rs`, `crates/alumina-markdown/tests/editor.rs`, `demo/src/markdown.rs`

- [ ] **Step 1: Write the failing tests**:
  - `read_only_click_reports_link`: `"[x](https://e.x)"` read-only, click at the screen
    position of char 1 (from the galley: `galley_pos + galley.pos_from_cursor(CCursor::new(1)).center()`)
    → `link_clicked == Some("https://e.x")`.
  - `editing_plain_click_places_cursor`: same, editing, plain click → `None`, cursor at
    char 1.
  - `editing_command_click_reports_link`: editing, click with `Modifiers::COMMAND` →
    `Some("https://e.x")`.
  - `click_outside_a_link`: read-only click on plain text → `None`.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement**: on `response.clicked()` (with `modifiers.command` when
  editing), the pointer's position → `galley.cursor_from_pos(pos - galley_pos)` → byte →
  the `Inline::Link` whose range holds it. Set the pointing-hand cursor icon when hovering a
  link where a click would report it. Show `last_link` in the demo.
- [ ] **Step 4: Run** — PASS.
- [ ] **Step 5: Commit** — `feat(alumina-markdown): report link clicks (#6)`

### Task 13: copying, the large document, and the #6 pull request

**Files:**
- Modify: `crates/alumina-markdown/tests/editor.rs`, `crates/alumina-markdown/tests/speed.rs`
- Create: `crates/alumina-markdown/README.md`, `crates/alumina-markdown/CHANGELOG.md`
- Modify: `crates/alumina-markdown/Cargo.toml` (`readme`, `keywords`, `categories` as
  alumina-datetime's; `publish = false` removed), `README.md` (the table row: "0.1"),
  `crates/alumina-markdown/src/lib.rs` (crate docs)

- [ ] **Step 1: Write `read_only_copies`**: read-only `"# a"`, select all
  (`COMMAND+A`), `COMMAND+C` (send `Event::Copy`) → the platform output's
  `copied_text == "# a"`.
- [ ] **Step 2: Run** — PASS or FAIL; fix in `editor.rs` if it fails.
- [ ] **Step 3: Add `layout_ten_thousand_lines`** (`#[ignore]`) to `speed.rs`: a kittest
  harness showing the 10,000-line document read-only in a `ScrollArea`; time 20 frames
  after a one-char change. Run with `--release -- --ignored --nocapture`. Under 16 ms a
  frame: done. Above: split the `LayoutJob` cache per block (cache each block's sections by
  its text, join them), measure again, and record both numbers in the PR.
- [ ] **Step 4: Write the README** (what it is, a usage snippet for the editor and the
  toolbar, the syntax covered, what is later, the no-ticking decision in one line) and the
  CHANGELOG's first entry; the crate docs in `lib.rs` as alumina-datetime's.
- [ ] **Step 5: Commit, push, PR** — `feat(alumina-markdown): read-only copying and the
  large-document check (#6)`; PR "alumina-markdown: reading", `Closes #6`, `Closes #1`. CI
  green; merge.
