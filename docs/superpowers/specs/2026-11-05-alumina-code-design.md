# alumina-code: a code editor that keeps the indentation

Asked for on 2026-11-05, as sapphire-turtle/alumina#28. Syntax highlighting comes after, in
#15, on top of this crate.

## Goal

An egui widget for editing code and settings files (YAML, TOML, JSON, Rust) in the sapphire
apps and corundum-framework's preview, where egui's `TextEdit` falls short: Enter puts a bare
`\n` (egui 0.37 leaves a TODO there for indenting it), Tab replaces the selection with one
`\t`, and Shift+Tab takes the indentation off the cursor's line alone.

Success means:

- in `alumina-demo` a block of code can be written without typing its indentation by hand:
  a new line keeps the indentation, one opened with a bracket goes in a level, a closing
  bracket comes back out, and lines are indented and outdented together;
- typing Japanese through an IME is untouched by any of it;
- the indentation is a value an app reads from its user's settings, written in a settings
  file through serde.

## Decisions

- **A `TextEdit` with keys taken before it**, as alumina-markdown does: the editor reads the
  frame's events, turns the ones it handles into edits on the string, and removes them, so
  egui keeps the cursor, the selection, the IME and undo.
- **No language.** The brackets `{ [ (` and `} ] )` are the only syntax it knows, so the
  same rules serve JSON, Rust, JS, C and the like. A bracket inside a string or a comment is
  taken as a bracket; a wrong level in is undone by the closing bracket coming back out.
  A trailing `:` (YAML, Python) is not looked at: without the language, `key: value` and
  `key:` look too alike.
- **The indentation unit is the app's**: `Indent::Spaces(n)` or `Indent::Tab`, `Spaces(4)`
  by default. It is passed every frame, so an app can take it from its settings and change
  it while the editor is open. Choosing it per language or per file is the app's.
- **Read-only is the same widget**, not editable; text can still be selected and copied,
  and Tab moves the focus on, as anywhere else.
- **Dependencies:** egui; serde behind the `serde` feature.
- **The small parts shared with alumina-markdown are copied**, not shared: the edit type
  (the range replaced, the text put there, the new selection) and the IME tracking, some
  tens of lines. When #15 has alumina-markdown depend on alumina-code they move here.

## Structure

```
crates/alumina-code/src/
  lib.rs      the public API; Indent
  indent.rs   edits on a string and a selection, no egui
  editor.rs   CodeEditor
```

### `Indent`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Indent {
    Tab,
    Spaces(u8),
}
// Default: Spaces(4)
```

In TOML that is `indent = "tab"` or `indent = { spaces = 2 }`. `Spaces(0)` is taken as
`Spaces(1)`.

A level of indentation is one tab or `n` spaces. Columns are counted in chars, a tab as 4,
which is how wide egui draws one.

### `indent`: the edits (no egui)

Pure functions from a source, a selection (a byte range on char boundaries) and an
`Indent` to an `Option<Edit>`. `None` when there is nothing to do, so the editor neither
counts a change nor makes an undo step. Lines end in `\n` or `\r\n`; a line break the
editor puts in is the one the source's first line ends with, `\n` if it has none.

- **Enter** (`newline`). The selection is replaced by a line break followed by the
  indentation of the line the selection starts on, up to the selection's start if that
  falls inside the indentation. If the last char before the selection on its line,
  past any spaces and tabs, is `{`, `[` or `(`, the new line goes one level in. If the
  first char after the selection on its line, past any spaces and tabs, is the matching
  closer, that closer and the rest of the line go to a line of their own after it, at the
  first line's indentation, and the cursor stays on the middle line.
- **A closing bracket** (`close`). A `}`, `]` or `)` typed with no selection, when
  everything before the cursor on its line is spaces and tabs and there is at least one
  level of it, takes one level off the end of that indentation before going in. Otherwise
  `None`, and egui types it.
- **Tab** (`indent`). A selection over two lines or more (a line the selection only
  reaches the start of does not count) puts one level at the start of each line it
  touches, blank lines left alone. Otherwise the selection is replaced by one level: a
  tab, or the spaces up to the next column that is a multiple of `n`.
- **Shift+Tab** (`outdent`). Each line the selection touches, or the cursor's line, loses
  one level from its start: a tab, or up to `n` spaces (up to 4 with `Indent::Tab`).
- After Tab and Shift+Tab over lines, the selection covers the same text it did, and an
  end at a line's start stays there.
- **Backspace** (`backspace`). With `Indent::Spaces(n)`, no selection, and only spaces
  before the cursor back to the line's start, it deletes back to the previous column that
  is a multiple of `n`. Otherwise `None`, and egui deletes a char.

### `CodeEditor`

```rust
let output = CodeEditor::new(&mut text)
    .id(id)                     // without it, an id of its own, as TextEdit's
    .read_only(false)
    .indent(settings.indent)    // Indent::Spaces(4) if not given
    .desired_rows(10)
    .accessible_label("Config")
    .show(ui);

output.changed;     // the text changed this frame
output.text_edit;   // egui's TextEditOutput
```

- The text is in the monospace font, the editor as wide as it is given, Tab kept in the
  editor while editing.
- It takes Enter, Tab, Shift+Tab, Backspace with no modifiers, and the text events `}`,
  `]`, `)`, and only while it has the focus and is not read-only.
- While an IME composition is open, it takes none of them; an Enter that commits a
  composition is the IME's.
- After an edit it scrolls to the cursor.
- `CodeOutput` is `#[non_exhaustive]`; every public item is documented.
- The text's look is made in one place in the editor, a plain monospace `LayoutJob`, so
  that #15 can put the highlighting there.

### The demo

A Code page: an editor on a sample in JSON and in Rust, a choice of the unit (2 spaces,
4 spaces, a tab), and a Read-only checkbox.

## Testing

- `indent`: unit tests per rule above, including a selection over several lines, blank
  lines, tabs and spaces mixed, `\r\n`, the start and the end of the source, and lines in
  Japanese and with emoji, checking the edits stay on char boundaries.
- `CodeEditor`: driven headless with egui_kittest, as alumina-markdown is: Enter after
  `{`, Enter in `{}`, `}` on an indented line, Tab and Shift+Tab over lines, Backspace,
  read-only taking no edits, and `Ime` events (Enter and `)` while composing go to the
  IME).
- `serde`: the TOML-shaped forms above round-trip (tested with serde_json's equivalent,
  `"tab"` and `{"spaces":2}`).
- By hand in the demo: a real IME, on a desktop and on a phone.

## Work

One pull request, closing #28, with the crate's README and CHANGELOG and the workspace
README's table brought up to date.
