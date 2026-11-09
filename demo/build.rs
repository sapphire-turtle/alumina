//! The demo's icon, from its SVGs alone: assets/app-icon.svg embedded in the Windows
//! executable, and the window's icon rendered to `OUT_DIR/app-icon.rgba` (raw RGBA,
//! `ICON_SIZE` pixels a side), which `src/lib.rs` includes. On macOS the window's icon is
//! assets/app-icon-macos.svg, the mark on its tile, as eframe gives it to the Dock too.

use std::path::Path;

const SVG: &str = "assets/app-icon.svg";
const MACOS_SVG: &str = "assets/app-icon-macos.svg";

fn main() {
    corundum_framework_build::embed_icon(SVG);
    println!("cargo:rerun-if-changed={SVG}");
    println!("cargo:rerun-if-changed={MACOS_SVG}");
    // The target's, not the host's: build.rs runs on the host.
    let macos = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "macos");
    // The window icon's size, in pixels a side; the Dock shows up to 512 on a Retina screen.
    let (svg_path, size) = if macos { (MACOS_SVG, 512) } else { (SVG, 256) };
    println!("cargo:rustc-env=ICON_SIZE={size}");
    let svg = std::fs::read(svg_path).expect("reading the icon SVG");
    let rgba = render_rgba(&svg, size);
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
