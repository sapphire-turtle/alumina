//! The demo's icon, from assets/app-icon.svg alone: embedded in the Windows executable,
//! and rendered for the window to `OUT_DIR/app-icon.rgba` (raw RGBA, `ICON_SIZE` pixels a
//! side), which `src/lib.rs` includes.

use std::path::Path;

const SVG: &str = "assets/app-icon.svg";
/// The window icon's size, in pixels a side. `src/lib.rs` reads it back from `ICON_SIZE`.
const ICON_SIZE: u32 = 256;

fn main() {
    corundum_framework_build::embed_icon(SVG);
    println!("cargo:rerun-if-changed={SVG}");
    println!("cargo:rustc-env=ICON_SIZE={ICON_SIZE}");
    let svg = std::fs::read(SVG).expect("reading the icon SVG");
    let rgba = render_rgba(&svg, ICON_SIZE);
    let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo");
    std::fs::write(Path::new(&out_dir).join("app-icon.rgba"), rgba).expect("writing the icon");
}

/// `svg` drawn centred on a `size`-pixel square, as straight (not premultiplied) RGBA.
fn render_rgba(svg: &[u8], size: u32) -> Vec<u8> {
    use resvg::{tiny_skia, usvg};

    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).expect("parsing the icon SVG");
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("a non-empty icon");
    let (w, h) = (tree.size().width(), tree.size().height());
    let scale = size as f32 / w.max(h);
    let transform = tiny_skia::Transform::from_translate(
        (size as f32 - w * scale) / 2.0,
        (size as f32 - h * scale) / 2.0,
    )
    .pre_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}
