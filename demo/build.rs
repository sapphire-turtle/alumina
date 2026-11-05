//! Embeds assets/app-icon.svg as the icon of the Windows executable.

fn main() {
    corundum_framework_build::embed_icon("assets/app-icon.svg");
}
