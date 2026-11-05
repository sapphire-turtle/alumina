# alumina-markdown

A Markdown editor for [egui](https://github.com/emilk/egui) that formats as you type.

The source is shown as it is, marks included (`#`, `**`, `- [ ]`, `` ` ``), drawn in a faint
colour; only its look changes. It is egui's `TextEdit` with a layouter, so the cursor, the
selection, the IME and undo are egui's own.

- **`MarkdownEditor`**: the editor. With `.read_only(true)` it is the same view, not
  editable, but its text can still be selected and copied.
- **`MarkdownToolbar`**: a row of buttons for the shortcuts' commands, placed by your app.
- **`parse`** and **`command`**: the pieces behind them, for reading a source into blocks
  and marks, or editing one, with no egui widget. **`layout`** and **`MarkdownStyle`** turn
  a parsed source into an egui `LayoutJob`.

```rust
use alumina_markdown::{MarkdownEditor, MarkdownToolbar, egui};

fn note(ui: &mut egui::Ui, note_id: u64, text: &mut String, read_only: bool) {
    // The toolbar finds the editor's selection by its id: give both the same one, and
    // each note its own. An editor with no toolbar needs no id.
    let id = ui.make_persistent_id(("note", note_id));
    if !read_only {
        MarkdownToolbar::new(text, id).show(ui);
    }
    let output = MarkdownEditor::new(text).id(id).read_only(read_only).show(ui);
    if output.changed {
        // Save `text`.
    }
    if let Some(url) = output.link_clicked {
        // Open `url`. The editor never opens anything itself.
    }
}
```

A link is clicked with a plain click when read-only, and with Ctrl / ⌘ + click while
editing, so that a plain click still places the cursor.

## Syntax

Headings (`#` to `######`, and setext), emphasis, strong, strikethrough, inline code, fenced
and indented code blocks, lists (bulleted, numbered, nested), task list items, block quotes
(nested), links (inline, autolinks, references), thematic breaks and YAML front matter
(the `---` block at the very start, shown as code).

## Highlighting

alumina knows no language: give the editor a `Highlighter` and it colours the code of
fenced blocks that name a language (the first word of the info string) and the front matter
(`"yaml"`), by the `CodeTheme` in `MarkdownStyle::code_theme`. Indented blocks and fences
without a language stay plain. The highlighter runs when the text, style or width changes,
not every frame; one swapped while the text is unchanged takes effect at the next change.

```rust,ignore
MarkdownEditor::new(&mut text).highlighter(&my_highlighter).show(ui);
```

## Keys

| Key | What it does |
|---|---|
| Ctrl / ⌘ + B | strong |
| Ctrl / ⌘ + I | emphasis |
| Ctrl / ⌘ + K | link |
| Ctrl / ⌘ + Shift + X | strikethrough |
| Ctrl / ⌘ + E | inline code |
| Enter in a list item | starts the next item (an empty one leaves the list) |
| Tab / Shift + Tab in a list item | indents / outdents the items selected |

They wrap the selection, or the word at the cursor, and take the mark off if it is there
already. Enter that commits an IME conversion is the IME's: it never also continues a list.

## Icons

The toolbar's buttons are labelled in ASCII (`ToolbarLabels`), and named for screen readers
and hover text in English (`ToolbarNames`, "Bold", "Bulleted list"): both are yours to
translate, as is the editor's own name (`MarkdownEditor::accessible_label`). alumina ships no
icons: give a button an `egui::ImageSource` with `ToolbarIcons`.

## Not yet

Tables, images shown inline, math, wiki links (`[[...]]`) and folding.

Ticking a `[ ]` by clicking is not done, read-only or editing: it would be an edit in a
widget that does not edit, or a click that does not place the cursor.

## Versions

| alumina-markdown | egui |
|---|---|
| 0.1 | 0.37 |

## Licence

MIT or Apache-2.0, at your option.
