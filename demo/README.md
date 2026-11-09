# alumina-demo

An app to try alumina's widgets by hand, on the desktops. It is a
workspace member, built and tested with the rest, and never published.

| Page | What it shows |
|---|---|
| Date & time | The picker buttons (date, period, time, timestamp), a calendar on its own with what it picked, a drum and the time drums, and the calendar beside the drums. Settings: Japanese words, the first day of the week, week numbers, the dots, the cell size, the time's format, and which kinds the calendar picks |
| Markdown | The editor (`MarkdownEditor`) on a sample, laid out as Markdown as you type, with its toolbar (`MarkdownToolbar`), YAML front matter and highlighted code blocks (the demo's own highlighters: YAML by hand, Rust over syntect), a Read-only checkbox and the last link clicked |
| Code | The editor (`CodeEditor`) on a JSON (plain), a YAML and a Rust sample, highlighted by the demo's own highlighters (YAML by hand, Rust over syntect), with a choice of unit (2 spaces, 4 spaces, Tab) and a Read-only checkbox |

Below 600 points wide (a narrow window, as a phone's) the settings fold away above the widgets and the parts
stack.

The window is eframe's, whose `system_fonts` finds a system font for the Japanese words.
The widget crates themselves depend on no window; nothing here, the demo included,
links corundum-framework, which these crates were split out of.

```sh
cargo run -p alumina-demo
```

It is for the desktops only, to stay small. The widgets are built and tested for iOS and
Android in Mobile CI; how they feel under a finger is tried in the apps that use them.

## The icon

`assets/app-icon.svg`, alumina's mark (`docs/logo.svg`) in the faceted brand blue:
`build.rs` renders it for the window, and embeds it in the Windows executable through
corundum-framework-build. That build-time helper is the one corundum crate here; nothing
of corundum is linked into the demo.

`assets/app-icon-macos.svg` is macOS's: the same mark on a light grey tile (`#F1F4F7`, the
sapphire-turtle avatar's) drawn on the macOS icon grid, as macOS sets a mark that does not
fill the grid on a plate of its own. On macOS `build.rs` renders it for the window instead,
which eframe also gives to the Dock.

Both are rendered at build time, so the repository keeps no PNG.

## CI

CI builds and tests the demo on the desktops.
