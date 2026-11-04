# alumina-markdown: a Markdown editor that formats as you type

Asked for on 2026-11-04, from sapphire-turtle/alumina#1 and its children #3, #4, #5, #6.

## Goal

An egui widget for writing Markdown in the sapphire apps (notes, the journal) and asteria
(descriptions), and for reading `.md` files in corundum-framework's preview
(sapphire-turtle/corundum-framework#97, #99). Success means, as #1 has it:

- in `alumina-demo` the first version's syntax can be written, changed and read;
- typing Japanese through an IME does not break the display while composing;
- corundum-framework-gui can offer it behind a feature flag. That flag is work in
  corundum-framework; here the crate only has to depend on nothing but egui and a parser,
  and have an API an app can call in a few lines.

## Decisions

- **One view.** The source is shown as it is, marks included (`#`, `**`, `- [ ]`, `` ` ``);
  only its look changes. The marks are drawn in a faint colour, never hidden.
- **A `TextEdit` with its own `layouter`.** The layouter turns the source into a
  `LayoutJob` with a format per range. Since what is shown is the source, character for
  character, egui keeps the cursor, the selection, the IME and undo.
- **Read-only is the same widget**, not editable, with the same look; text can still be
  selected and copied.
- **The parser is pulldown-cmark.** It is small, its `into_offset_iter` gives each event
  the byte range of the source it came from, and with `ENABLE_STRIKETHROUGH` and
  `ENABLE_TASKLISTS` it covers the first version. comrak covers all of GFM but builds an
  AST on every parse and gives lines and columns, not byte offsets; nothing in the first
  version needs what it adds.
- **The whole source is parsed again when it changes**, and the result is cached against
  the text, so a frame with no change does not parse. #3 measures a 10,000-line document;
  only if that is too slow for a keystroke do we reparse just the changed blocks.
- **Dependencies:** egui and pulldown-cmark. No corundum.
- **Ticking a checkbox by clicking is not done.** In read-only it would mean an edit in a
  widget that does not edit; while editing, with `[ ]` drawn as text and not as a box, a
  click that ticks instead of placing the cursor would surprise. It is to be looked at
  again if the editor ever moves towards WYSIWYG, as Obsidian does (marks hidden outside
  the line being edited, `[ ]` drawn as a box).

### The first version's syntax

Headings (ATX `#`…`######`, and setext), emphasis, strong, strikethrough, inline code,
fenced and indented code blocks, lists (bulleted, numbered, nested), task list items, block
quotes (nested), links (inline, autolinks, reference links as pulldown-cmark resolves them),
thematic breaks.

Later, not in this work: tables, images shown inline, math, wiki links (`[[...]]`), folding,
syntax highlighting in code blocks.

## Structure

```
crates/alumina-markdown/src/
  lib.rs        the public API
  parse.rs      #3  source -> Doc
  style.rs      #4  MarkdownStyle
  layout.rs     #4  Doc + MarkdownStyle -> LayoutJob, cached by the text's hash
  editor.rs     #4, #6  MarkdownEditor
  commands.rs   #5  edits on a string and a selection, no egui
  toolbar.rs    #5  MarkdownToolbar
```

### `parse`: the intermediate form (#3)

`parse(source: &str) -> Doc`. Every range is a `Range<usize>` of bytes in the source, on
char boundaries.

- `Doc::blocks: Vec<Block>`, in source order, nested blocks after their parent:
  `Block { kind: BlockKind, range, marks: Vec<Range<usize>> }`, where `BlockKind` is
  `Paragraph`, `Heading { level }`, `ListItem { depth, number: Option<u64>, task:
  Option<bool> }`, `Quote { depth }`, `CodeBlock { info: Option<String> }` or
  `ThematicBreak`. `marks` are the ranges of the block's own marks: the `#`s, the bullet or
  the number, the `[ ]`, the `>`s, the fences.
- `Doc::inlines: Vec<Inline>`: `Inline { kind: InlineKind, range, marks }`, where
  `InlineKind` is `Emphasis`, `Strong`, `Strikethrough`, `Code`, or `Link { url: String,
  text: Range<usize> }`. `marks` are the delimiters: `*`, `**`, `~~`, the backticks, the
  `[`, `](url)` of a link.

pulldown-cmark does not report where a mark is, only the range of the whole element; the
marks are found by looking at the source at the element's edges (the delimiter run at
each end of an emphasis, the prefix of a list item's first line, the `>`s at the start of
each line of a quote).

### `style` and `layout`: the look (#4)

`MarkdownStyle` holds the body font, the heading sizes (one per level), the monospace
font, and the colours: text, faint marks, links, quotes, the code background. It is plain
data the app may change; `MarkdownStyle::from_visuals(&Visuals)` gives the defaults, so it
follows the dark and light themes.

`layout(source, &Doc, &MarkdownStyle, wrap_width) -> LayoutJob`. It walks the source once,
giving each byte range the `TextFormat` of the blocks and inlines over it: a heading's
size, strong's weight, emphasis's italics, strikethrough's line, code's monospace and
background, a link's colour and underline, a quote's colour, a mark's faint colour. Inside
a code block nothing is read as Markdown. A thematic break is shown as its marks; the
layouter cannot draw a line across the width, so the editor paints one over that row.

The job is cached in egui's memory against the hash of the source, the style and the wrap
width, as egui's own code editor example does.

### `MarkdownEditor` (#4, #6)

```rust
let output = MarkdownEditor::new(&mut text)
    .id_salt("note")
    .read_only(false)
    .style(MarkdownStyle::from_visuals(ui.visuals()))
    .desired_rows(10)
    .show(ui);

output.changed;        // the text changed this frame
output.link_clicked;   // Option<String>: the URL of a link clicked; the app opens it
output.text_edit;      // egui's TextEditOutput: the response, the cursor, the galley
```

- **Links.** Read-only: a click on a link. While editing: a Ctrl/⌘+click, so that a plain
  click still places the cursor. The editor finds the link under the pointer through the
  galley's cursor at that point and the `Doc`; it never opens anything itself.
- **Large documents (#6).** Measured with the 10,000-line document read-only. If laying out
  the whole text each change is too slow, the job is split per block and only the changed
  ones are made again; drawing only the visible rows is the step after that.

### `commands` and `MarkdownToolbar` (#5)

`commands` are pure functions from a source and a selection (a byte range) to an edit (the
range replaced, the text put there, the new selection). They have no egui in them, so they
are tested with strings alone.

- Inline marks on the selection, or on the word at the cursor if nothing is selected:
  strong, emphasis, strikethrough, code, link (`[text](url)`, the selection placed on
  `url`). If the mark is there already, it is taken off.
- Line marks on every line the selection touches: heading level (1–6, or none), bulleted
  list, numbered list, task list, quote, code block (fences around the lines). Again they
  toggle.
- Lists: Enter at the end of an item starts the next (the number one up, a task's
  `[ ]` unticked); Enter in an empty item leaves the list (the empty marker is removed);
  Tab and Shift+Tab indent and outdent the items the selection touches.

The editor consumes the keys before its `TextEdit` sees them: Ctrl/⌘+B (strong), +I
(emphasis), +K (link), +Shift+X (strikethrough), +E (code), and Enter, Tab and Shift+Tab
where the cursor is in a list item.

`MarkdownToolbar::new(&mut text, editor_id).labels(ToolbarLabels { .. }).show(ui)` is a
row of buttons for the same commands. It is a widget of its own, so the app places it; the
labels are strings the app may replace. It reads the editor's selection from its
`TextEditState` by id, applies the command, and writes the new selection back, so the
editor shows it on the next frame.

### The demo

The demo's Markdown page gets an editor with a sample covering every piece of the first
version, the toolbar above it, a read-only switch, and the last link clicked. It comes in
with #4 and grows with #5 and #6.

## Testing

- `parse` and `commands`: unit tests per piece of syntax and per command, with sources in
  Japanese and with emoji, checking byte ranges stay on char boundaries.
- The editor and the toolbar: driven headless with egui_kittest, as alumina-datetime's
  widgets are: typing, Enter in a list, the shortcuts, a toolbar click, a link click in
  read-only and with the modifier while editing, and `Ime` events (composing Japanese,
  then committing). With `ALUMINA_RENDER_DIR` they render as the other widgets' tests do.
- Speed: a test that parses and lays out the 10,000-line document and reports the time,
  run with `--ignored`, so CI does not depend on a machine's speed.
- By hand in the demo: typing Japanese with a real IME, on a desktop and on a phone.

## Work

One pull request per child issue, in this order, each closing its issue:

1. #3: `parse`, its tests, the measurement.
2. #4: `style`, `layout`, `MarkdownEditor` with read-only, the demo page.
3. #5: `commands`, the keys, `MarkdownToolbar`.
4. #6: link clicks, selection and copy in read-only, the large-document measurement and
   what it calls for. Closing it closes #1, with alumina-markdown's README and the
   workspace README's table brought up to date.
