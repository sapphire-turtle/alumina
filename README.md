<img src="docs/logo.svg" alt="" width="72" height="72">

# alumina

Reusable [egui](https://github.com/emilk/egui) widgets from the sapphire apps and asteria,
split out of [corundum-framework](https://github.com/sapphire-turtle/corundum-framework) so
any egui app can use them. They depend on egui, not on corundum or a window.

| Crate | What it is | Status |
|---|---|---|
| [`alumina-code`](crates/alumina-code) | A code editor that keeps the indentation: Enter, brackets, Tab and Shift+Tab over lines, highlighted through the app's highlighter | 0.1 |
| [`alumina-datetime`](crates/alumina-datetime) | A calendar (a day, a week, a month, a year or a range), time drums, a timestamp picker, and buttons that open them in a popup | 0.1 |
| [`alumina-markdown`](crates/alumina-markdown) | A Markdown editor that formats as you type, with shortcuts, a toolbar and list completion | 0.1 |
| [`alumina-demo`](demo) | A desktop app to try them all | not released |

## Trying them

```sh
cargo run -p alumina-demo
```

[`demo/README.md`](demo/README.md) says what each page shows.

## Development

- `cargo test --workspace`: the unit tests and the widgets driven headless with
  [egui_kittest](https://crates.io/crates/egui_kittest) (clicks, keys, popups). With
  `ALUMINA_RENDER_DIR=<dir>` each widget test also renders its window there as a PNG
  (on the GPU, or a software Vulkan such as lavapipe).
- CI: clippy and the tests on Linux, macOS and Windows; Mobile CI runs clippy on the
  libraries for iOS and Android and their tests on a simulator and an emulator.
- [RELEASING.md](RELEASING.md): the releases, with release-plz.

## Licence

MIT or Apache-2.0, at your option.
