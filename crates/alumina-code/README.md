# alumina-code

A code editor for [egui](https://github.com/emilk/egui) that keeps the indentation.

It is egui's `TextEdit` in a monospace font, with Enter, Tab, Shift+Tab, Backspace and the
closing brackets taken for the indentation. The cursor, the selection, the IME and undo are
egui's own.

- **`CodeEditor`**: the editor. With `.read_only(true)` it is the same view, not editable,
  but its text can still be selected and copied.
- **`Indent`**: the unit of indentation, `Tab` or `Spaces(n)` (`Spaces(4)` by default). It is
  your app's setting: the editor keeps none.
- **`newline`**, **`indent`**, **`outdent`**, **`backspace`** and **`close`**: the edits
  behind the keys, as functions on a source and a byte selection, with no egui widget.

```rust
use alumina_code::{CodeEditor, Indent};
use alumina_code::egui;

fn editor(ui: &mut egui::Ui, source: &mut String, indent: Indent) {
    let out = CodeEditor::new(source)
        .indent(indent)
        .desired_rows(20)
        .show(ui);
    if out.changed {
        // Save `source`.
    }
}
```

## Keys

| Key | What it does |
|---|---|
| Enter | breaks the line and keeps its indentation; goes a level in after `{`, `[` or `(`; between a matching pair (`{|}`, `[|]`, `(|)`), the closer moves to a line of its own |
| Tab | indents: within a line, to the next stop; over a selection of several lines, each line a level in, blank lines left alone |
| Shift + Tab | outdents each line the selection touches by a level |
| Backspace in the indentation | steps back to the previous multiple of the unit, not one space (not for `Tab`) |
| `}`, `]` or `)` on a blank, indented line | comes out a level as it is typed |

Enter that commits an IME conversion is the IME's: it never also breaks the line. A
read-only editor takes no keys.

The editor takes keys only while it has the focus and only as far as it can keep the text in
the order typed: in a frame with several keystrokes, it stops taking keys after the first
event it leaves to egui (typed text, a paste, a key it does not handle), and the rest go to
egui as they are.

## Settings

Brackets are not language-aware: `{ [ (` and `} ] )` are the only syntax the editor knows,
inside strings and comments too, so the same rules serve JSON, Rust, JS, C and the like.

With the `serde` feature, `Indent` can be read from (and written to) your settings. In TOML:

```toml
indent = "tab"
```

```toml
indent = { spaces = 2 }
```

## Highlighting

The editor knows no language: your app implements `Highlighter`, which turns a language name
and some code into byte ranges of `TokenKind`s (return `None` for a language you do not
know). The colours follow the theme: `CodeTheme::from_visuals` reads dark or light from the
`ui`, or give your own with `.theme(..)`. The highlighter runs when the text, language, theme,
width or font changes, not every frame; a highlighter swapped for another takes effect at the
next such change. Ranges it returns are cut at the end of the code; empty, reversed or off-char-boundary
ranges are dropped; later ranges win. A `Highlighter` is `Send + Sync + RefUnwindSafe`, so a
cache inside it needs a `Mutex` or atomics.

```rust
use std::ops::Range;
use alumina_code::{CodeEditor, Highlighter, TokenKind};
use alumina_code::egui;

struct Fns;
impl Highlighter for Fns {
    fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>> {
        (language == "rust").then(|| {
            code.match_indices("fn").map(|(i, m)| (i..i + m.len(), TokenKind::Keyword)).collect()
        })
    }
}

fn editor(ui: &mut egui::Ui, source: &mut String) {
    CodeEditor::new(source).language("rust").highlighter(&Fns).show(ui);
}
```

## Versions

| alumina-code | egui |
|---|---|
| 0.1 | 0.37 |

## Licence

MIT or Apache-2.0, at your option.
