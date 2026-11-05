---
title: Alumina sample
date: 2026-11-05
tags:
  - egui
  - markdown
draft: false
---

# Alumina 見出し🎌

Setext heading
==============

Second level
------------

Plain text with *emphasis*, **strong**, _under_, __double__, ~~strike~~ and `code`.
日本語の文章にも **強調** と *斜体* が使えます🎌

A [link](https://example.com), an autolink <https://example.org>, and a [reference][ref].

- bullet one
- bullet two
  ```yaml
  # settings
  name: "alumina"
  debug: false
  ```
  1. nested number
  2. another **number**
- [ ] open task
- [x] done task

1. first
2. second

> quote
> > nested quote with `code`

```rust
// Not bold, not markdown: code.
fn main() {
    let a = "**not bold**";
    println!("{a}");
}
```

    indented code

---

[ref]: https://example.net
