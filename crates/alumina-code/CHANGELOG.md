# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/sapphire-turtle/alumina/releases/tag/alumina-code-v0.1.0) - 2026-11-05

### Added

- *(alumina-code)* highlighting through the app's Highlighter ([#15](https://github.com/sapphire-turtle/alumina/pull/15))
- *(alumina-code)* the demo's Code page, docs and CI ([#28](https://github.com/sapphire-turtle/alumina/pull/28))
- *(alumina-code)* CodeEditor ([#28](https://github.com/sapphire-turtle/alumina/pull/28))
- *(alumina-code)* Tab, Shift+Tab and Backspace edits ([#28](https://github.com/sapphire-turtle/alumina/pull/28))
- *(alumina-code)* the crate, Indent, and Enter and closing-bracket edits ([#28](https://github.com/sapphire-turtle/alumina/pull/28))

### Fixed

- *(alumina-code)* Highlighter is Send + Sync + RefUnwindSafe; docs, hardening and tests (#15, #14)
- *(alumina-code)* a highlighted empty text keeps its row ([#15](https://github.com/sapphire-turtle/alumina/pull/15))
- *(alumina-code)* winit's key before text, CRLF cursors, and doc and test fixes ([#28](https://github.com/sapphire-turtle/alumina/pull/28))
- *(alumina-code)* keep the typed order, and test read-only Tab and undo ([#28](https://github.com/sapphire-turtle/alumina/pull/28))
- *(alumina-code)* Tab indents a whole-line selection ([#28](https://github.com/sapphire-turtle/alumina/pull/28))
- *(alumina-code)* a closer comes out only from a whole level ([#28](https://github.com/sapphire-turtle/alumina/pull/28))

### Other

- *(alumina-code)* [**breaking**] Highlighter has no thread bounds ([#15](https://github.com/sapphire-turtle/alumina/pull/15))

### Added

- *(alumina-code)* `CodeEditor`, a code editor that keeps the indentation, and `Indent`, the app's unit, readable from settings with the `serde` feature
- *(alumina-code)* syntax highlighting through the app's `Highlighter`, coloured by `CodeTheme`
