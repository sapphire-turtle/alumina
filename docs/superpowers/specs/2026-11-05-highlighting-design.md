# Syntax highlighting through a trait, and front matter

Asked for on 2026-11-05, as sapphire-turtle/alumina#15 and #14.

## Goal

Colour in `CodeEditor` and in alumina-markdown's code blocks and front matter, without
alumina knowing any language: alumina is a library of egui widgets, so it must not grow a
lexer, weigh more, or stop building for wasm, and a code widget that knows two languages
would be lopsided. The app brings the highlighter; alumina asks it what each range of the
code is, and colours it.

Success means:

- an app can give `CodeEditor` and `MarkdownEditor` a highlighter of its own in a few
  lines, and alumina still depends on nothing but egui (and pulldown-cmark for markdown);
- with no highlighter both look as they do today;
- a Markdown note's YAML front matter reads as YAML, and its fenced code as its language;
- the demo shows two ways of writing a highlighter: one by hand (YAML) and one over an
  existing crate (Rust, over syntect).

## Decisions

- **A trait, returning kinds of token.** The highlighter says what each range is
  (`TokenKind`); the colours are alumina's, from the theme, so every highlighter looks
  the same and follows dark and light. An app can replace the colours. A highlighter
  cannot set fonts or weights: this is not a full editor.
- **The language is a string** the highlighter interprets: the app's choice for
  `CodeEditor`, the first word of a fence's info string in Markdown, `yaml` for front
  matter. Aliases (`yml`, `rs`) are the highlighter's business. `None` back means "not
  mine": the code is shown plain.
- **Results are cached** against the text, the language and the theme, so the
  highlighter runs only when the text changes. A highlighter swapped while the text stays
  the same takes effect at the next change; this is documented.
- **Whole text each time.** A change highlights the whole code again (or, in Markdown,
  every code block). Parsing only what changed is for later, if the measurement calls
  for it.
- **alumina-markdown depends on alumina-code** for the trait and the colours.
- **Front matter is YAML only**: `---` … `---` at the very start of the document, through
  pulldown-cmark's `ENABLE_YAML_STYLE_METADATA_BLOCKS`. TOML's `+++` is left for later.
- **The helpers copied in #28** (the edit type, IME tracking) stay copied: sharing them
  would make internal functions public API. That is a separate issue.
- **The demo's highlighters are the app's**, in the demo crate, which is never
  published; syntect and its dependencies reach only the demo.

## alumina-code

```rust
pub trait Highlighter {
    /// What each range of `code` is, or `None` if `language` is not one it knows.
    fn highlight(&self, language: &str, code: &str) -> Option<Vec<(Range<usize>, TokenKind)>>;
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Comment, Keyword, Type, Function, String, Number, Constant, Key, Punctuation, Attribute,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodeTheme { /* a colour per TokenKind */ }
impl CodeTheme {
    pub fn from_visuals(visuals: &Visuals) -> Self;
    pub fn color(&self, kind: TokenKind) -> Color32;
    pub fn set_color(&mut self, kind: TokenKind, color: Color32);
}

CodeEditor::new(&mut text)
    .language("rust")
    .highlighter(&highlighter)   // &dyn Highlighter; none: plain, as today
    .theme(theme)                // CodeTheme::from_visuals(ui.visuals()) if not given
    .show(ui);
```

- `Constant` is `true`, `false`, `null` and the like; `Key` a key in YAML, TOML or JSON;
  `Function` a function's or a macro's name; `Attribute` an attribute or an annotation
  (`#[derive]`, a YAML anchor or tag).
- The ranges are bytes of `code`. alumina takes them as they come: out of range, reversed
  or off a char boundary is clamped or dropped, overlaps are painted in order (a later one
  wins), and nothing panics.
- `CodeTheme::from_visuals` picks colours that read on both the dark and the light
  background; text with no token keeps the text colour.
- The colouring goes in the editor's single `layout_job` place, which #28 left for it.

## alumina-markdown

- `MarkdownEditor::highlighter(&dyn Highlighter)`; `MarkdownStyle` gains
  `code_theme: CodeTheme`, from the same visuals.
- **Code blocks.** A fenced block with an info string has its first word passed as the
  language; its code is the block's content, without the fences and, in a list or a
  quote, without the container's prefixes on each line (the pieces pulldown-cmark gives as
  text, joined), and the ranges that come back are mapped to the source. Indented blocks
  and fences with no info string are not highlighted.
- **Front matter (#14).** `BlockKind::FrontMatter`: the block at the start of the source
  from `---` to the closing `---` (or `...`), with the two delimiter lines as its marks.
  It looks like a code block (monospace, background), its delimiters faint, and with a
  highlighter its content is highlighted as `yaml`. A `---` anywhere else is a thematic
  break or a setext underline as before; an unclosed front matter while typing does not
  break the rest of the document.
- The highlighting is part of the cached layout, so a frame with no change does not run
  the highlighter.

## The demo

- `demo/src/highlight/yaml.rs`: by hand, a line-based lexer for keys, strings (plain,
  quoted), numbers, `true`/`false`/`null`/`~`, comments, anchors, aliases and tags,
  `---`/`...`, list dashes and flow punctuation. Enough for a settings file, not the whole
  of YAML.
- `demo/src/highlight/rust.rs`: over syntect (`default-features = false`, features
  `default-syntaxes` and `regex-fancy`: pure Rust, no oniguruma), mapping its scopes to
  `TokenKind` (`comment` → Comment, `keyword`/`storage` → Keyword, `entity.name.type`
  and `support.type` → Type, `entity.name.function`/`support.function`/macros →
  Function, `string` → String, `constant.numeric` → Number, `constant.language` →
  Constant, `meta.attribute` → Attribute, `punctuation` → Punctuation). The syntax set is
  loaded once.
- One `DemoHighlighter` answering `yaml`/`yml` and `rust`/`rs`, handed to both pages.
- The Code page gets a YAML sample beside JSON and Rust; JSON has no highlighter, which
  shows the plain fallback.
- The Markdown page's sample starts with a YAML front matter and has a Rust and a YAML
  code block, one of them in a list.
- cargo-deny: syntect needs bincode 1, flagged unmaintained; an ignore with its reason,
  reaching only the demo, as the existing ones do.

## Testing

- alumina-code: a highlighter in the tests colours what it says (the galley's sections
  carry the theme's colours), `None` and no highlighter leave it plain, bad ranges do not
  panic, the highlighter runs once for unchanged text over many frames.
- alumina-markdown: front matter parsed and laid out as said, `---` elsewhere unchanged,
  unclosed front matter while typing; a code block's language reaches the highlighter;
  ranges in a block inside a list or a quote land on the right bytes; an indented block
  is not highlighted.
- The demo's highlighters: unit tests on small inputs (YAML by hand: each kind; Rust over
  syntect: a keyword, a string, a comment, a function).
- Speed: the 10,000-line Markdown measurement again, with a highlighter, reported.

## Work

One pull request closing #15 and #14, with the READMEs, CHANGELOGs and the demo README
brought up to date.
