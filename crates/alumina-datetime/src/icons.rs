//! The app's icons for the buttons that can be one.

use egui::{Button, Image, ImageSource, Response, Role, Ui, WidgetInfo};

/// An image for each button that can be one; a button without one keeps its look (the
/// drawn arrows for [`previous`](Self::previous) and [`next`](Self::next), the
/// [`Labels`](crate::Labels) text for the others).
///
/// With an icon the button shows it, and the `Labels` text becomes its accessible name and
/// its hover text. alumina ships no icons and no image loader: pass the sources and install
/// the loader they need (an SVG one for Lucide) on the `Context`. Pass them with
/// [`CalendarConfig::icons`](crate::CalendarConfig::icons).
#[derive(Clone, Debug, Default)]
pub struct CalendarIcons {
    /// Previous month / year / decade (`Labels::previous`).
    pub previous: Option<ImageSource<'static>>,
    /// Next month / year / decade (`Labels::next`).
    pub next: Option<ImageSource<'static>>,
    /// The Today button (`Labels::today`).
    pub today: Option<ImageSource<'static>>,
    /// The Clear button (`Labels::clear`).
    pub clear: Option<ImageSource<'static>>,
    /// The Now button of the time and timestamp pickers (`Labels::now`).
    pub now: Option<ImageSource<'static>>,
}

/// A small button showing `label`, or `icon` when there is one (then `label` is its
/// accessible name and its hover text).
pub(crate) fn small_button(
    ui: &mut Ui,
    label: &str,
    icon: &Option<ImageSource<'static>>,
) -> Response {
    let Some(source) = icon else {
        return ui.small_button(label);
    };
    let size = ui.spacing().interact_size.y - 2.0 * ui.spacing().button_padding.y;
    let image = Image::new(source.clone())
        .fit_to_exact_size(egui::vec2(size, size))
        .tint(ui.visuals().text_color());
    let response = ui.add(Button::image(image)).on_hover_text(label);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), label));
    response
}
