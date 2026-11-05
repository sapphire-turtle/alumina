# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- *(alumina-markdown)* a document starting with `---` ... `---` is now read as front matter, where it was a thematic break and a setext heading
- *(alumina-markdown)* depends on alumina-code, whose `Highlighter`, `TokenKind` and `CodeTheme` it re-exports

### Added

- *(alumina-markdown)* YAML front matter, shown as code
- *(alumina-markdown)* code blocks and front matter coloured by the app's Highlighter

## [0.1.0](https://github.com/sapphire-turtle/alumina/releases/tag/alumina-markdown-v0.1.0) - 2026-11-04

### Added

- *(alumina-markdown)* the API before publishing ([#6](https://github.com/sapphire-turtle/alumina/pull/6))
- *(alumina-markdown)* read-only copying and the large-document check ([#6](https://github.com/sapphire-turtle/alumina/pull/6))
- *(alumina-markdown)* report link clicks ([#6](https://github.com/sapphire-turtle/alumina/pull/6))
- *(alumina-markdown)* MarkdownToolbar ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* the editor's shortcuts and list keys ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* continue, leave and indent lists ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* toggle line marks: headings, lists, quotes, code ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* toggle inline marks on a selection ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* MarkdownEditor, a TextEdit laid out as Markdown ([#4](https://github.com/sapphire-turtle/alumina/pull/4))
- *(alumina-markdown)* lay out a Doc with a format per range ([#4](https://github.com/sapphire-turtle/alumina/pull/4))
- *(alumina-markdown)* parse inlines with their delimiters ([#3](https://github.com/sapphire-turtle/alumina/pull/3))
- *(alumina-markdown)* parse blocks with their marks ([#3](https://github.com/sapphire-turtle/alumina/pull/3))

### Fixed

- *(alumina-markdown)* an id per editor, scrolling to edits, the toolbar from the keyboard ([#6](https://github.com/sapphire-turtle/alumina/pull/6))
- *(alumina-markdown)* Ctrl+K leaves images alone and reads one line ([#6](https://github.com/sapphire-turtle/alumina/pull/6))
- *(alumina-markdown)* the toolbar never slices a stale selection ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* exact shortcuts, and a composing flag that cannot stick ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* parse once per list command ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* no edit when a line command has nothing to do ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* unwrap only a matching pair, and links too ([#5](https://github.com/sapphire-turtle/alumina/pull/5))
- *(alumina-markdown)* parse once per change, for the rules too ([#4](https://github.com/sapphire-turtle/alumina/pull/4))
- *(alumina-markdown)* marks of fences, setext lines and rules in containers ([#3](https://github.com/sapphire-turtle/alumina/pull/3))

### Other

- *(alumina-markdown)* draw the rules in one pass, only those in view ([#6](https://github.com/sapphire-turtle/alumina/pull/6))
- *(alumina-markdown)* time a 10,000-line parse ([#3](https://github.com/sapphire-turtle/alumina/pull/3))
- *(alumina-markdown)* not published until the editor is written ([#1](https://github.com/sapphire-turtle/alumina/pull/1))
- the workspace: alumina-datetime, alumina-markdown and alumina-demo

### Added

- *(alumina-markdown)* `MarkdownEditor`, a Markdown editor that formats as you type, and a read-only view with the same look
- *(alumina-markdown)* shortcuts, Enter and Tab in lists, and `MarkdownToolbar`
- *(alumina-markdown)* link clicks reported to the app
- *(alumina-markdown)* accessible names for the editor and the toolbar's buttons (`ToolbarNames`)
