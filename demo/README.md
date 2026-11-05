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

The window is corundum-framework-gui's, which finds a CJK font for the Japanese words.
The widget crates themselves depend on neither corundum nor a window.

```sh
cargo run -p alumina-demo
```

It is for the desktops only, to stay small. The widgets are built and tested for iOS and
Android in Mobile CI; how they feel under a finger is tried in the apps, or in
corundum-framework's demo, whose window the apps share.

## The icon

`assets/app-icon.svg`, alumina's mark (`docs/logo.svg`) in the faceted brand blue: the
window's, and the Windows executable's through `build.rs`.

## CI

CI builds and tests the demo on the desktops.
