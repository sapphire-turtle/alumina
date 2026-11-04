//! [`MarkdownToolbar`]: buttons that run the [`commands`](crate::commands) on the editor.

use egui::{
    Button, Context, Id, Image, ImageSource, Key, KeyboardShortcut, Modifiers, Response, Role, Ui,
    WidgetInfo,
};

use crate::commands::{Command, command};
use std::ops::Range;

use crate::editor::{apply, selection, set_selection};

macro_rules! fields {
    ($($(#[$m:meta])* $name:ident),* $(,)?) => {
        /// The text of each button, also its hover text and accessible name.
        #[derive(Clone, Debug)]
        pub struct ToolbarLabels { $(pub $name: String,)* }

        /// An image for each button; a button without one shows its label.
        ///
        /// alumina ships no icons and no image loader: pass the sources and install
        /// the loader they need (an SVG one for Lucide) on the `Context`.
        #[derive(Clone, Debug, Default)]
        pub struct ToolbarIcons { $(pub $name: Option<ImageSource<'static>>,)* }
    };
}

fields!(
    strong,
    emphasis,
    strikethrough,
    code,
    link,
    heading,
    bullet_list,
    numbered_list,
    task_list,
    quote,
    code_block,
);

impl Default for ToolbarLabels {
    /// ASCII only: egui's default fonts lack glyphs such as `☐`.
    fn default() -> Self {
        let s = String::from;
        Self {
            strong: s("B"),
            emphasis: s("I"),
            strikethrough: s("S"),
            code: s("</>"),
            link: s("Link"),
            heading: s("H"),
            bullet_list: s("-"),
            numbered_list: s("1."),
            task_list: s("[ ]"),
            quote: s(">"),
            code_block: s("{ }"),
        }
    }
}

/// A row of buttons for the [`MarkdownEditor`](crate::MarkdownEditor) with the same id.
pub struct MarkdownToolbar<'t> {
    text: &'t mut String,
    editor_id: Id,
    labels: ToolbarLabels,
    icons: ToolbarIcons,
}

impl<'t> MarkdownToolbar<'t> {
    pub fn new(text: &'t mut String, editor_id: Id) -> Self {
        Self {
            text,
            editor_id,
            labels: ToolbarLabels::default(),
            icons: ToolbarIcons::default(),
        }
    }

    pub fn labels(mut self, labels: ToolbarLabels) -> Self {
        self.labels = labels;
        self
    }

    pub fn icons(mut self, icons: ToolbarIcons) -> Self {
        self.icons = icons;
        self
    }

    /// `changed()` on the response when a button edited the text.
    pub fn show(self, ui: &mut Ui) -> Response {
        let Self {
            text,
            editor_id,
            labels,
            icons,
        } = self;
        let ctx = ui.ctx().clone();
        // An editor that lost the focus to a press on a button collapsed its selection
        // as it did: take the selection back, and the focus with it.
        if let Some(sel) = ctx.data_mut(|d| d.remove_temp::<Range<usize>>(restore_id(editor_id))) {
            set_selection(&ctx, editor_id, text, sel);
            ui.memory_mut(|m| m.request_focus(editor_id));
        }
        let editor_focused = ui.memory(|m| m.has_focus(editor_id));
        if editor_focused {
            let sel = selection(&ctx, editor_id, text);
            ctx.data_mut(|d| match sel {
                Some(sel) => {
                    d.insert_temp(held_id(editor_id), sel);
                }
                None => {
                    d.remove_temp::<Range<usize>>(held_id(editor_id));
                }
            });
        }
        let cmd_key = |key| Some(KeyboardShortcut::new(Modifiers::COMMAND, key));
        let heading = Command::Heading(next_heading(text, &ctx, editor_id));
        let buttons = [
            (
                Command::Strong,
                &labels.strong,
                &icons.strong,
                cmd_key(Key::B),
            ),
            (
                Command::Emphasis,
                &labels.emphasis,
                &icons.emphasis,
                cmd_key(Key::I),
            ),
            (
                Command::Strikethrough,
                &labels.strikethrough,
                &icons.strikethrough,
                Some(KeyboardShortcut::new(
                    Modifiers::COMMAND.plus(Modifiers::SHIFT),
                    Key::X,
                )),
            ),
            (Command::Code, &labels.code, &icons.code, cmd_key(Key::E)),
            (Command::Link, &labels.link, &icons.link, cmd_key(Key::K)),
            (heading, &labels.heading, &icons.heading, None),
            (
                Command::BulletList,
                &labels.bullet_list,
                &icons.bullet_list,
                None,
            ),
            (
                Command::NumberedList,
                &labels.numbered_list,
                &icons.numbered_list,
                None,
            ),
            (Command::TaskList, &labels.task_list, &icons.task_list, None),
            (Command::Quote, &labels.quote, &icons.quote, None),
            (
                Command::CodeBlock,
                &labels.code_block,
                &icons.code_block,
                None,
            ),
        ];
        let mut changed = false;
        let mut pressed = false;
        let mut response = ui
            .horizontal(|ui| {
                for (cmd, label, icon, shortcut) in buttons {
                    let hover = match shortcut {
                        Some(s) => format!("{label} ({})", ui.ctx().format_shortcut(&s)),
                        None => label.clone(),
                    };
                    let response = button(ui, label, icon).on_hover_text(hover);
                    pressed |= response.is_pointer_button_down_on();
                    if response.clicked() {
                        changed |= run(ui, text, editor_id, cmd);
                        pressed = true;
                    }
                }
            })
            .response;
        if !editor_focused && !pressed {
            ctx.data_mut(|d| d.remove_temp::<Range<usize>>(held_id(editor_id)));
        }
        if changed {
            response.mark_changed();
        }
        response
    }
}

/// One button: the icon alone if there is one, else the label.
fn button(ui: &mut Ui, label: &str, icon: &Option<ImageSource<'static>>) -> Response {
    let Some(source) = icon else {
        return ui.add(Button::new(label));
    };
    let size = ui.spacing().interact_size.y - 2.0 * ui.spacing().button_padding.y;
    let image = Image::new(source.clone())
        .fit_to_exact_size(egui::vec2(size, size))
        .tint(ui.visuals().text_color());
    let response = ui.add(Button::image(image));
    response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), label));
    response
}

/// Where the toolbar keeps the editor's selection while a button is held.
fn held_id(editor: Id) -> Id {
    editor.with("toolbar-held")
}

/// Where the toolbar keeps the selection to give the editor back next frame.
fn restore_id(editor: Id) -> Id {
    editor.with("toolbar-restore")
}

/// The selection a button acts on: the editor's, or what it was when the button was
/// pressed; the end of the text if it never had one.
fn target(ctx: &Context, id: Id, text: &str) -> Range<usize> {
    let held = ctx.data(|d| d.get_temp::<Range<usize>>(held_id(id)));
    held.or_else(|| selection(ctx, id, text))
        .unwrap_or(text.len()..text.len())
}

/// Apply `cmd` to the editor's selection as one undo step, and give the editor the focus
/// back. Whether the text changed.
fn run(ui: &Ui, text: &mut String, id: Id, cmd: Command) -> bool {
    let ctx = ui.ctx();
    let edit = command(text, target(ctx, id, text), cmd);
    if let Some(edit) = &edit {
        apply(ctx, id, text, edit);
        // The editor may drop its selection this frame (see `show`).
        ctx.data_mut(|d| d.insert_temp(restore_id(id), edit.selection.clone()));
    }
    ctx.data_mut(|d| d.remove_temp::<Range<usize>>(held_id(id)));
    ui.memory_mut(|m| m.request_focus(id));
    ctx.request_repaint();
    edit.is_some()
}

/// The heading level the heading button sets next: none, 1, 2, 3, none.
fn next_heading(text: &str, ctx: &Context, id: Id) -> u8 {
    let at = target(ctx, id, text).start;
    let line = &text[text[..at].rfind('\n').map_or(0, |i| i + 1)..];
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    let marked = matches!(
        line.as_bytes().get(hashes),
        None | Some(b' ' | b'\t' | b'\r' | b'\n')
    );
    match if marked { hashes } else { 0 } {
        1 => 2,
        2 => 3,
        3.. => 0,
        _ => 1,
    }
}
