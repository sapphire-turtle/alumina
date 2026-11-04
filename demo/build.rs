//! The app icon, from the SVGs in assets/: embedded in the Windows executable, and rendered
//! to the PNGs the phones' projects take (ignored by git) when cargo builds for one.
//!
//! The phones' half is corundum-framework-build's `ios_app_icon` and `android_app_icons` in
//! the corundum release after 0.9.1 (corundum-framework#112); the demo, on corundum 0.7,
//! keeps a copy here until it moves to that.

use std::path::Path;

fn main() {
    corundum_framework_build::embed_icon("assets/app-icon.svg");
    ios_app_icon(
        "assets/app-icon-square.svg",
        "ios/Assets.xcassets/AppIcon.appiconset",
    );
    android_app_icons(
        "assets/app-icon.svg",
        "assets/app-icon-foreground.svg",
        "android/app/src/main/res",
    );
}

/// Android's screen densities, as each mipmap folder's name and its scale against mdpi in
/// halves (mdpi 1, hdpi 1.5, xhdpi 2, xxhdpi 3, xxxhdpi 4).
const ANDROID_DENSITIES: [(&str, u32); 5] = [
    ("mdpi", 2),
    ("hdpi", 3),
    ("xhdpi", 4),
    ("xxhdpi", 6),
    ("xxxhdpi", 8),
];

fn target_os() -> String {
    std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default()
}

/// iOS's icon: `svg_path` full-bleed (iOS rounds the corners), 1024 pixels with no alpha
/// channel (which the App Store refuses), as `AppIcon.png` in the asset catalog's icon set.
fn ios_app_icon(svg_path: &str, appiconset_dir: &str) {
    println!("cargo:rerun-if-changed={svg_path}");
    if target_os() != "ios" {
        return;
    }
    let result = read(svg_path)
        .and_then(|svg| opaque_png(&svg, 1024))
        .and_then(|png| write_if_changed(&Path::new(appiconset_dir).join("AppIcon.png"), &png));
    if let Err(what) = result {
        println!("cargo:warning=could not render the iOS app icon: {what}");
    }
}

/// Android's icons in every density: `legacy_svg` as `ic_launcher.png` (48 dp, before
/// Android 8) and `foreground_svg` as `ic_launcher_foreground.png` (108 dp, the adaptive
/// icon's foreground).
fn android_app_icons(legacy_svg: &str, foreground_svg: &str, res_dir: &str) {
    println!("cargo:rerun-if-changed={legacy_svg}");
    println!("cargo:rerun-if-changed={foreground_svg}");
    if target_os() != "android" {
        return;
    }
    let layers = [
        (legacy_svg, "ic_launcher.png", 24),
        (foreground_svg, "ic_launcher_foreground.png", 54),
    ];
    let result = layers
        .into_iter()
        .try_for_each(|(svg_path, name, half_dp)| {
            let svg = read(svg_path)?;
            ANDROID_DENSITIES.iter().try_for_each(|(density, halves)| {
                let png = corundum_framework_platform::icon::render_png(&svg, half_dp * halves)
                    .map_err(|e| format!("rendering {svg_path}: {e}"))?;
                let path = Path::new(res_dir)
                    .join(format!("mipmap-{density}"))
                    .join(name);
                write_if_changed(&path, &png)
            })
        });
    if let Err(what) = result {
        println!("cargo:warning=could not render the Android app icons: {what}");
    }
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("reading {path}: {e}"))
}

/// Writes `bytes` to `path`, making its folder, unless the file already holds them, so
/// Xcode and Gradle do not see a change on every build. It asks Cargo to rerun when the file
/// changes too, so one deleted is written again on the next build.
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    println!("cargo:rerun-if-changed={}", path.display());
    if std::fs::read(path).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))
}

/// Renders `svg` to an RGB PNG of `size` pixels, its transparency flattened onto black.
fn opaque_png(svg: &[u8], size: u32) -> Result<Vec<u8>, String> {
    let rgba = corundum_framework_platform::icon::render_rgba(svg, size)
        .map_err(|e| format!("rendering: {e}"))?;
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let a = u16::from(p[3]);
            [0, 1, 2].map(|i| ((u16::from(p[i]) * a + 127) / 255) as u8)
        })
        .collect();
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size, size);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| format!("encoding: {e}"))?;
    writer
        .write_image_data(&rgb)
        .map_err(|e| format!("encoding: {e}"))?;
    writer.finish().map_err(|e| format!("encoding: {e}"))?;
    Ok(out)
}
