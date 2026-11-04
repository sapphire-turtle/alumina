# alumina-code Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new crate `alumina-code` with `CodeEditor`, an egui `TextEdit` for code that keeps and completes the indentation (Enter, brackets, Tab, Shift+Tab, Backspace), and a Code page in the demo.

**Architecture:** Pure edit functions in `indent.rs` (string + byte selection + `Indent` → `Edit`), and `editor.rs`, which takes the keys it handles out of the frame's events before its `TextEdit` sees them and applies the edits as one undo step each — the pattern of `crates/alumina-markdown/src/editor.rs`, whose helpers are copied.

**Tech Stack:** Rust 2024, egui 0.37, optional serde, egui_kittest 0.37 for tests.

**Spec:** `docs/superpowers/specs/2026-11-05-alumina-code-design.md`

## Global Constraints

- Dependencies: `egui = { workspace = true }`; `serde = { version = "1", default-features = false, features = ["derive"], optional = true }` behind the feature `serde`. Nothing else. No eframe, no corundum.
- `#![warn(missing_docs)]`; every public item documented. Public output structs `#[non_exhaustive]`.
- All ranges are bytes on char boundaries; a selection out of range, reversed or off a char boundary is clamped (as `alumina_markdown::commands::clamp`).
- A level is one `\t` or `n` spaces; `Indent::Spaces(0)` acts as `Spaces(1)`. Columns count chars, a `\t` as 4.
- An edit function with nothing to do returns `None`.
- The line break the editor inserts is the one the source's first line ends with (`\r\n` or `\n`), `\n` if it has none.
- Commit messages: `feat(alumina-code): … (#28)`, ending with the `Co-Authored-By` line the session gives.
- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass after every task.

## Review Focus

1. A Tab or Shift+Tab the edit functions return `None` for (a multi-line selection of blank lines, an unindented line) must still be taken: egui's own Tab handler deletes the selection first. — test in Task 3.
2. Enter or `)` during an IME composition must reach the IME untouched. — test in Task 3.
3. Multi-byte text (Japanese, emoji) before the cursor: columns count chars, edits stay on char boundaries. — tests in Tasks 1 and 2.
4. `\r\n` sources: inserted breaks match, and a line's end is found before the `\r`. — tests in Tasks 1 and 2.
5. Read-only: none of the keys edit, and Tab is not kept in the editor. — test in Task 3.

---

### Task 1: The crate, `Indent`, and the Enter and closing-bracket edits

**Files:**
- Create: `crates/alumina-code/Cargo.toml`, `crates/alumina-code/src/lib.rs`, `crates/alumina-code/src/indent.rs`
- Modify: `Cargo.toml` (workspace `members`, and `[workspace.dependencies] alumina-code = { version = "0.1.0", path = "crates/alumina-code" }`)

**Interfaces:**
- Produces, re-exported from `lib.rs`:
  - `pub enum Indent { Tab, Spaces(u8) }`, derives `Clone, Copy, Debug, PartialEq, Eq, Hash`, `Default` = `Spaces(4)`, with `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]`.
  - `pub struct Edit { pub replace: Range<usize>, pub with: String, pub selection: Range<usize> }` with `pub fn apply(&self, text: &mut String)` — as `alumina_markdown::Edit`.
  - `pub fn newline(source: &str, selection: Range<usize>, indent: Indent) -> Edit`
  - `pub fn close(source: &str, selection: Range<usize>, closer: char, indent: Indent) -> Option<Edit>`
  - `pub use egui;`

`Cargo.toml` of the crate: copy the metadata shape of `crates/alumina-markdown/Cargo.toml` (description "A code editor for egui that keeps the indentation", keywords `["egui", "code", "editor", "text-editor", "gui"]`); `[features] serde = ["dep:serde"]`; dev-dependencies `serde_json = "1"` plus markdown's three (egui with `default_fonts`, egui_kittest, egui_extras not needed — leave it out).

Abbreviations in the tests below: `const S4: Indent = Indent::Spaces(4);`, `fn ed(r: Range<usize>, w: &str, s: Range<usize>) -> Edit`.

- [ ] **Step 1: Write the failing tests** in `indent.rs` `mod tests`

```rust
#[test] fn enter_keeps_the_indentation() {
    assert_eq!(newline("    let x = 1;", 14..14, S4), ed(14..14, "\n    ", 19..19));
}
#[test] fn enter_inside_the_indentation_keeps_what_is_before_the_cursor() {
    assert_eq!(newline("        x", 4..4, S4), ed(4..4, "\n    ", 9..9));
}
#[test] fn enter_after_an_opener_goes_in() {
    assert_eq!(newline("fn a() {", 8..8, S4), ed(8..8, "\n    ", 13..13));
    assert_eq!(newline("  x(  ", 6..6, S4), ed(6..6, "\n      ", 13..13)); // spaces after `(`
    assert_eq!(newline("[", 1..1, Indent::Tab), ed(1..1, "\n\t", 3..3));
}
#[test] fn enter_between_brackets_puts_the_closer_on_its_own_line() {
    assert_eq!(newline("a {}", 3..3, S4), ed(3..3, "\n    \n", 8..8));
    // The spaces between the cursor and the closer go.
    assert_eq!(newline("  x(  )", 4..4, S4), ed(4..6, "\n      \n  ", 11..11));
    // Not a pair: only the level in.
    assert_eq!(newline("a {)", 3..3, S4), ed(3..3, "\n    ", 8..8));
}
#[test] fn enter_replaces_the_selection() {
    assert_eq!(newline("ab cd", 2..3, S4), ed(2..3, "\n", 3..3));
}
#[test] fn enter_counts_bytes_in_japanese() {
    let s = "  日本 {"; // 10 bytes
    assert_eq!(newline(s, 10..10, S4), ed(10..10, "\n      ", 17..17));
}
#[test] fn enter_matches_crlf() {
    assert_eq!(newline("x\r\n  y", 6..6, S4), ed(6..6, "\r\n  ", 10..10));
    assert_eq!(newline("a {}\r\n", 3..3, S4), ed(3..3, "\r\n    \r\n", 9..9));
}
#[test] fn a_closer_on_a_blank_indented_line_comes_out_a_level() {
    assert_eq!(close("        ", 8..8, '}', S4), Some(ed(4..8, "}", 5..5)));
    assert_eq!(close("      ", 6..6, ']', S4), Some(ed(4..6, "]", 5..5))); // back to a multiple of 4
    assert_eq!(close("\t\t", 2..2, ')', Indent::Tab), Some(ed(1..2, ")", 2..2)));
    assert_eq!(close("    ", 4..4, ')', Indent::Tab), Some(ed(0..4, ")", 1..1))); // up to 4 spaces
    assert_eq!(close("  \t", 3..3, '}', S4), Some(ed(2..3, "}", 3..3))); // a tab at the end goes
    assert_eq!(close("x\n    ", 6..6, '}', S4), Some(ed(2..6, "}", 3..3)));
}
#[test] fn a_closer_elsewhere_is_typed_by_egui() {
    assert_eq!(close("  x", 3..3, '}', S4), None);
    assert_eq!(close("", 0..0, '}', S4), None);
    assert_eq!(close("    ", 0..4, '}', S4), None);
    assert_eq!(close("    ", 4..4, 'x', S4), None);
}
#[test] fn spaces_zero_is_one() {
    assert_eq!(close("  ", 2..2, '}', Indent::Spaces(0)), Some(ed(1..2, "}", 2..2)));
}
#[cfg(feature = "serde")]
#[test] fn indent_reads_from_settings() {
    assert_eq!(serde_json::to_string(&Indent::Tab).unwrap(), r#""tab""#);
    assert_eq!(serde_json::to_string(&Indent::Spaces(2)).unwrap(), r#"{"spaces":2}"#);
    assert_eq!(serde_json::from_str::<Indent>(r#"{"spaces":2}"#).unwrap(), Indent::Spaces(2));
    assert_eq!(Indent::default(), S4);
}
```

- [ ] **Step 2: Run and see them fail**

Run: `cargo test -p alumina-code --features serde`
Expected: does not compile (`newline`, `close`, `Indent` missing).

- [ ] **Step 3: Implement** `Indent`, `Edit`, `newline`, `close` and a private `clamp` (copy `alumina_markdown`'s) per the spec's "Enter" and "A closing bracket" rules. Openers and closers: `{`/`}`, `[`/`]`, `(`/`)`. "One level off the end": a trailing `\t`; else the trailing spaces back to the previous column that is a multiple of `n` (with `Indent::Tab`, of 4), at least one, spaces only. The crate docs in `lib.rs` say what the crate is, with a `no_run` example of `CodeEditor` added in Task 3 (for now, describe the functions).

- [ ] **Step 4: Run them pass**

Run: `cargo test -p alumina-code --features serde && cargo test -p alumina-code`
Expected: PASS (the serde test runs only in the first).

- [ ] **Step 5: Commit** `feat(alumina-code): the crate, Indent, and Enter and closing-bracket edits (#28)`

---

### Task 2: Tab, Shift+Tab and Backspace edits

**Files:**
- Modify: `crates/alumina-code/src/indent.rs`, `crates/alumina-code/src/lib.rs` (re-exports)

**Interfaces:**
- Consumes: `Indent`, `Edit`, `clamp` from Task 1.
- Produces:
  - `pub fn indent(source: &str, selection: Range<usize>, indent: Indent) -> Option<Edit>`
  - `pub fn outdent(source: &str, selection: Range<usize>, indent: Indent) -> Option<Edit>`
  - `pub fn backspace(source: &str, selection: Range<usize>, indent: Indent) -> Option<Edit>`

"The lines the selection touches": from the line of `start` to the line of `end`, leaving out the line of `end` when `end` is at its start and `end > start`. "Over two lines or more" means that set has two lines or more. After an edit over lines, each selection end moves with the text at it, except one at a line's start, which stays at that line's start. A whole-lines edit's `replace` is the range from the first touched line's start to the last's end.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn tab_inserts_up_to_the_next_stop() {
    assert_eq!(indent("ab", 1..1, S4), Some(ed(1..1, "   ", 4..4)));
    assert_eq!(indent("ab", 0..0, S4), Some(ed(0..0, "    ", 4..4)));
    assert_eq!(indent("日本", 6..6, S4), Some(ed(6..6, "  ", 8..8)));
    assert_eq!(indent("\tx", 2..2, S4), Some(ed(2..2, "   ", 5..5))); // column 5
    assert_eq!(indent("abc", 1..2, S4), Some(ed(1..2, "   ", 4..4)));
    assert_eq!(indent("ab", 1..1, Indent::Tab), Some(ed(1..1, "\t", 2..2)));
}
#[test] fn tab_over_lines_indents_each_but_the_blank() {
    assert_eq!(indent("a\nb\n\nc", 0..6, S4), Some(ed(0..6, "    a\n    b\n\n    c", 0..18)));
    assert_eq!(indent("a\n  \nb", 0..6, Indent::Tab), Some(ed(0..6, "\ta\n  \n\tb", 0..8)));
}
#[test] fn tab_leaves_a_line_the_selection_only_reaches() {
    assert_eq!(indent("a\nb\nc", 0..4, S4), Some(ed(0..3, "    a\n    b", 0..12)));
    assert_eq!(indent("a\nb\nc", 1..4, S4), Some(ed(0..3, "    a\n    b", 5..12)));
}
#[test] fn tab_over_blank_lines_only_does_nothing() {
    assert_eq!(indent("\n\n", 0..2, S4), None);
}
#[test] fn tab_over_crlf_lines() {
    assert_eq!(indent("a\r\nb", 0..4, S4), Some(ed(0..4, "    a\r\n    b", 0..12)));
}
#[test] fn shift_tab_takes_a_level_off_each_line() {
    assert_eq!(outdent("    a", 5..5, S4), Some(ed(0..4, "", 1..1)));
    assert_eq!(outdent("  a", 3..3, S4), Some(ed(0..2, "", 1..1)));
    assert_eq!(outdent("      a", 7..7, S4), Some(ed(0..4, "", 3..3)));
    assert_eq!(outdent("\ta", 2..2, S4), Some(ed(0..1, "", 1..1)));
    assert_eq!(outdent("      a", 7..7, Indent::Tab), Some(ed(0..4, "", 3..3)));
    assert_eq!(outdent("    a", 2..2, S4), Some(ed(0..4, "", 0..0))); // inside the indent
    assert_eq!(outdent("    a\n  b\nc", 0..11, S4), Some(ed(0..11, "a\nb\nc", 0..5)));
    assert_eq!(outdent("  日本\n  x", 2..11, S4), Some(ed(0..12, "日本\nx", 0..7)));
}
#[test] fn shift_tab_on_unindented_lines_does_nothing() {
    assert_eq!(outdent("a\nb", 0..3, S4), None);
}
#[test] fn backspace_in_the_indentation_takes_a_level() {
    assert_eq!(backspace("        ", 8..8, S4), Some(ed(4..8, "", 4..4)));
    assert_eq!(backspace("      x", 6..6, S4), Some(ed(4..6, "", 4..4)));
    assert_eq!(backspace("    x", 4..4, S4), Some(ed(0..4, "", 0..0)));
    assert_eq!(backspace(" ", 1..1, S4), Some(ed(0..1, "", 0..0)));
    assert_eq!(backspace("a\n      ", 8..8, Indent::Spaces(2)), Some(ed(6..8, "", 6..6)));
}
#[test] fn backspace_elsewhere_is_egui_s() {
    assert_eq!(backspace("  a  ", 5..5, S4), None);
    assert_eq!(backspace("\t  ", 3..3, S4), None);
    assert_eq!(backspace("    ", 4..4, Indent::Tab), None);
    assert_eq!(backspace("    ", 0..4, S4), None);
    assert_eq!(backspace("    ", 0..0, S4), None);
}
```

- [ ] **Step 2: Run and see them fail** — `cargo test -p alumina-code`: does not compile.
- [ ] **Step 3: Implement** the three functions per the spec's Tab, Shift+Tab and Backspace rules and the definitions above. Re-export them from `lib.rs`.
- [ ] **Step 4: Run them pass** — `cargo test -p alumina-code --features serde`: PASS; clippy clean.
- [ ] **Step 5: Commit** `feat(alumina-code): Tab, Shift+Tab and Backspace edits (#28)`

---

### Task 3: `CodeEditor`

**Files:**
- Create: `crates/alumina-code/src/editor.rs`, `crates/alumina-code/tests/editor.rs`
- Modify: `crates/alumina-code/src/lib.rs` (re-export, crate docs with a `no_run` example)

**Interfaces:**
- Consumes: everything from Tasks 1 and 2.
- Produces:
  - `pub struct CodeEditor<'t>` with `new(&'t mut String)`, `.id(Id)`, `.read_only(bool)`, `.indent(Indent)`, `.desired_rows(usize)` (default 8), `.accessible_label(impl Into<String>)` (default `"Code"`), `.show(&mut Ui) -> CodeOutput`.
  - `#[non_exhaustive] pub struct CodeOutput { pub changed: bool, pub text_edit: TextEditOutput }` with a hand-written `Debug` (as `MarkdownOutput`'s).

Copy from `crates/alumina-markdown/src/editor.rs`, adapting: `char_to_byte`, `byte_to_char`, `selection`, `char_range`, `apply` (one undo step), `composing_id`, `forget_composing`, `take_ime_enters`, `scroll_to_cursor`, the id fallback in `show`, the accesskit label, and `changed: !read_only && response.changed()`. Leave out links, rules, the toolbar's `edited_id`, the cache.

What `take_keys` takes, only when editable, focused and enabled (as markdown's `takes_keys`):

| Event | Modifiers (`matches_exact`) | Edit | When the edit is `None` |
|---|---|---|---|
| `Key::Enter` pressed | `NONE` | `newline` | — (always `Some`); not taken while composing or with IME events this frame |
| `Key::Tab` pressed | `NONE` | `indent` | still removed from the events |
| `Key::Tab` pressed | `SHIFT` | `outdent` | still removed from the events |
| `Key::Backspace` pressed | `NONE` | `backspace` | left for egui |
| `Event::Text(t)` where `t` is exactly `"}"`, `"]"` or `")"` | — | `close` | left for egui |

While a composition is open (or the frame has IME events), it takes none of these. The `TextEdit` is `TextEdit::multiline(..)` with `.font(egui::TextStyle::Monospace)`, `.lock_focus(!read_only)`, `.desired_width(f32::INFINITY)`, `.desired_rows(..)`, `.id(id)`, and a layouter calling one private `fn layout_job(ui: &Ui, text: &str, wrap_width: f32) -> LayoutJob` (`LayoutJob::simple` with the monospace font and the `TextEdit`'s default text colour) — the single place #15 will add highlighting. Read-only shows `&mut &str`, as markdown does.

- [ ] **Step 1: Write the failing tests** in `tests/editor.rs`, with a harness like markdown's `editor()` (`Harness::builder().with_size([400.0, 200.0]).build_ui_state`, `MissingGlyphPolicy::Tofu`, `Id::unique("code")`, state `{ text, changed, read_only, indent }`), a `focus` helper clicking `Role::MultilineTextInput`, a `set_cursor(h, char_index)` helper storing a `TextEditState` cursor, and `key(h, Key, Modifiers)` / `text(h, &str)` helpers pushing `Event`s then `h.run()`:

  - `enter_after_a_brace_goes_in`: text `"fn a() {}"`, cursor 8, Enter → `"fn a() {\n    \n}"`; then typing `"x"` → `"fn a() {\n    x\n}"` (the cursor was on the middle line). `changed` is true.
  - `a_closer_comes_out`: `"{\n        "`, cursor at end, `text("}")` → `"{\n    }"`.
  - `tab_and_shift_tab_over_lines`: `"a\nb"`, select chars 0..3, Tab → `"    a\n    b"`; Shift+Tab → `"a\nb"`.
  - `tab_on_blank_lines_is_taken`: `"\n\n"`, select 0..2, Tab → text unchanged (`"\n\n"`), and the editor still has the focus.
  - `shift_tab_on_an_unindented_selection_keeps_it`: `"ab"`, select 0..2, Shift+Tab → `"ab"` unchanged.
  - `tab_keeps_the_focus`: `"x"`, cursor 1, Tab → `"x   "`, and `h.ctx.memory(|m| m.focused())` is the editor's id.
  - `backspace_takes_a_level`: `"        "`, cursor 8, Backspace → `"    "`.
  - `undo_reverts_one_edit`: `"{}"`, cursor 1, Enter, then Command+Z → `"{}"`.
  - `ime_keeps_enter_and_closers`: `"{}"`, cursor 1; push `Ime(Preedit("か"))`, `Key Enter`, `Ime(Commit("か"))` in one frame → `"{か}"` (no newline); and on `"    "`, cursor 4, one frame of `Ime(Preedit("x"))`, `Ime(Commit("x"))`, `Text(")")` → `"    x)"` (no dedent in a frame with IME events).
  - `read_only_takes_nothing`: read-only, `"{}"`, focus, cursor 1, Enter, Tab, `text("}")` → text still `"{}"`, `changed` false.
  - `the_indent_unit_is_the_app_s`: state indent `Indent::Tab`, `"{}"`, cursor 1, Enter → `"{\n\t\n}"`.

- [ ] **Step 2: Run and see them fail** — `cargo test -p alumina-code --test editor`: does not compile.
- [ ] **Step 3: Implement** `editor.rs` as described; re-export `CodeEditor`, `CodeOutput`; finish the crate docs in `lib.rs` with a `no_run` example (an app passing `settings.indent`).
- [ ] **Step 4: Run them pass** — `cargo test -p alumina-code --all-targets --features serde`, clippy, `cargo doc -p alumina-code --no-deps` with no warnings.
- [ ] **Step 5: Commit** `feat(alumina-code): CodeEditor (#28)`

---

### Task 4: The demo page, docs and CI

**Files:**
- Create: `demo/src/code.rs`, `demo/src/code-sample.json`, `demo/src/code-sample.rs.txt`, `crates/alumina-code/README.md`, `crates/alumina-code/CHANGELOG.md`
- Modify: `demo/Cargo.toml` (`alumina-code = { workspace = true }`), `demo/src/lib.rs` (`Page::Code`, "Code" tab after "Markdown"), `demo/README.md` (a Code row in the pages table), `README.md` (a row for `alumina-code`: "A code editor that keeps the indentation: Enter, brackets, Tab and Shift+Tab over lines" | 0.1), `.github/workflows/ci.yml` (add `alumina-code` to the `libraries-without-a-window` loop; in `post-test` add `cargo test -p alumina-code --features serde --locked`), `.github/workflows/mobile-ci.yml` (add `-p alumina-code` beside each `-p alumina-markdown`)

**Interfaces:**
- Consumes: `CodeEditor`, `Indent` from Tasks 1–3.

`CodeDemo`: two samples (JSON, Rust) each with its own `String`, a selectable for which is shown, a choice of unit (`2 spaces`, `4 spaces`, `Tab`) defaulting to 2 for JSON and 4 for Rust when switching sample, a Read-only checkbox, the editor in a vertical `ScrollArea` with `desired_rows(20)` and `accessible_label("Code")`. The Rust sample is `.rs.txt` so the workspace does not try to read it as code. The crate README follows `crates/alumina-markdown/README.md`'s layout (what it is, an example, what each key does, the `serde` feature with the TOML forms, what is not done yet: highlighting, #15). The CHANGELOG follows markdown's, with one Added line: `*(alumina-code)* CodeEditor, a code editor that keeps the indentation, and Indent, the app's unit, readable from settings with the serde feature`.

- [ ] **Step 1: Write** the demo page and wire it in.
- [ ] **Step 2: Run** `cargo build -p alumina-demo --bin alumina-demo && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo publish --workspace --dry-run --locked`: all pass (`cargo tree -p alumina-code -e normal` shows no eframe or corundum).
- [ ] **Step 3: Write** the README, CHANGELOG and CI changes.
- [ ] **Step 4: Run** the same commands again: pass.
- [ ] **Step 5: Commit** `feat(alumina-code): the demo's Code page, docs and CI (#28)`
