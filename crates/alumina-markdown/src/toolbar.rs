//! [`MarkdownToolbar`]: buttons that run the [`commands`](crate::commands) on the editor.

use std::ops::Range;

use egui::text::CCursorRange;
use egui::text_edit::TextEditState;
use egui::{Button, Context, Id, Image, ImageSource, Response, Role, Ui, WidgetInfo};

use crate::commands::{Command, clamp, command, heading_level};
use crate::editor::{apply, char_to_byte, mark_edited, selection, set_selection, shortcut};

macro_rules! fields {
    ($($(#[$m:meta])* $name:ident),* $(,)?) => {
        /// The visible text of each button, used when it has no icon.
        ///
        /// Short and ASCII by default; what screen readers say is [`ToolbarNames`].
        #[derive(Clone, Debug)]
        #[non_exhaustive]
        pub struct ToolbarLabels { $($(#[$m])* pub $name: String,)* }

        /// The accessible name of each button: what screen readers say, and the hover
        /// text before the shortcut (`Bold (Ctrl+B)`).
        #[derive(Clone, Debug)]
        #[non_exhaustive]
        pub struct ToolbarNames { $($(#[$m])* pub $name: String,)* }

        /// An image for each button; a button without one shows its label.
        ///
        /// alumina ships no icons and no image loader: pass the sources and install
        /// the loader they need (an SVG one for Lucide) on the `Context`.
        #[derive(Clone, Debug, Default)]
        #[non_exhaustive]
        pub struct ToolbarIcons { $($(#[$m])* pub $name: Option<ImageSource<'static>>,)* }
    };
}

fields!(
    /// Strong (`**`).
    strong,
    /// Emphasis (`*`).
    emphasis,
    /// Strikethrough (`~~`).
    strikethrough,
    /// Inline code (`` ` ``).
    code,
    /// Link (`[text](url)`).
    link,
    /// Heading: none, `#`, `##`, `###`, none.
    heading,
    /// Bulleted list (`- `).
    bullet_list,
    /// Numbered list (`1. `).
    numbered_list,
    /// Task list (`- [ ] `).
    task_list,
    /// Block quote (`> `).
    quote,
    /// Fenced code block.
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

impl Default for ToolbarNames {
    /// In English.
    fn default() -> Self {
        let s = String::from;
        Self {
            strong: s("Bold"),
            emphasis: s("Italic"),
            strikethrough: s("Strikethrough"),
            code: s("Code"),
            link: s("Link"),
            heading: s("Heading"),
            bullet_list: s("Bulleted list"),
            numbered_list: s("Numbered list"),
            task_list: s("Task list"),
            quote: s("Quote"),
            code_block: s("Code block"),
        }
    }
}

/// A row of buttons for the [`MarkdownEditor`](crate::MarkdownEditor) with the same id.
///
/// It edits the text whatever the editor's state: an app hides it next to a read-only
/// editor.
#[derive(Debug)]
pub struct MarkdownToolbar<'t> {
    text: &'t mut String,
    editor_id: Id,
    labels: ToolbarLabels,
    names: ToolbarNames,
    icons: ToolbarIcons,
}

impl<'t> MarkdownToolbar<'t> {
    /// A toolbar for the editor of `text` whose id is `editor_id` (set with
    /// [`MarkdownEditor::id`](crate::MarkdownEditor::id)).
    pub fn new(text: &'t mut String, editor_id: Id) -> Self {
        Self {
            text,
            editor_id,
            labels: ToolbarLabels::default(),
            names: ToolbarNames::default(),
            icons: ToolbarIcons::default(),
        }
    }

    /// The visible text of the buttons; defaults to [`ToolbarLabels::default`].
    pub fn labels(mut self, labels: ToolbarLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The accessible names of the buttons; defaults to [`ToolbarNames::default`], in
    /// English.
    pub fn names(mut self, names: ToolbarNames) -> Self {
        self.names = names;
        self
    }

    /// Images for the buttons, in place of their labels; none by default.
    pub fn icons(mut self, icons: ToolbarIcons) -> Self {
        self.icons = icons;
        self
    }

    /// Shows the buttons in a row. `changed()` on the response when a button edited the
    /// text.
    pub fn show(self, ui: &mut Ui) -> Response {
        let Self {
            text,
            editor_id,
            labels,
            names,
            icons,
        } = self;
        let ctx = ui.ctx().clone();
        // An editor that lost the focus to a press on a button collapsed its selection
        // as it did: take the selection back, and the focus with it.
        let now = ctx.cumulative_pass_nr();
        let restore = ctx.data_mut(|d| d.remove_temp::<(u64, Range<usize>)>(restore_id(editor_id)));
        if let Some((stamp, sel)) = restore
            && now <= stamp + 2
        {
            set_selection(&ctx, editor_id, text, clamp(text, sel));
            ui.memory_mut(|m| m.request_focus(editor_id));
        }
        // A held selection is only good while a button is down, or one of ours has the
        // keyboard focus (reached by Tab from the editor).
        let was_pressed = ctx.data(|d| d.get_temp::<bool>(pressed_id(editor_id)).unwrap_or(false));
        let ours = ctx.data(|d| {
            d.get_temp::<Vec<Id>>(buttons_id(editor_id))
                .unwrap_or_default()
        });
        let focused = ui.memory(|m| m.focused());
        let editor_focused = focused == Some(editor_id);
        let bar_focused = focused.is_some_and(|f| ours.contains(&f));
        if !was_pressed && !editor_focused && !bar_focused {
            ctx.data_mut(|d| d.remove_temp::<CCursorRange>(held_id(editor_id)));
        }
        if editor_focused {
            // Chars, as egui keeps them: no walk over the text on an idle frame.
            let sel = TextEditState::load(&ctx, editor_id).and_then(|s| s.cursor.char_range());
            ctx.data_mut(|d| match sel {
                Some(sel) => {
                    d.insert_temp(held_id(editor_id), sel);
                }
                None => {
                    d.remove_temp::<CCursorRange>(held_id(editor_id));
                }
            });
        }
        let buttons = [
            (
                Command::Strong,
                &labels.strong,
                &names.strong,
                &icons.strong,
            ),
            (
                Command::Emphasis,
                &labels.emphasis,
                &names.emphasis,
                &icons.emphasis,
            ),
            (
                Command::Strikethrough,
                &labels.strikethrough,
                &names.strikethrough,
                &icons.strikethrough,
            ),
            (Command::Code, &labels.code, &names.code, &icons.code),
            (Command::Link, &labels.link, &names.link, &icons.link),
            // The level is read from the text when the button is pressed.
            (
                Command::Heading(0),
                &labels.heading,
                &names.heading,
                &icons.heading,
            ),
            (
                Command::BulletList,
                &labels.bullet_list,
                &names.bullet_list,
                &icons.bullet_list,
            ),
            (
                Command::NumberedList,
                &labels.numbered_list,
                &names.numbered_list,
                &icons.numbered_list,
            ),
            (
                Command::TaskList,
                &labels.task_list,
                &names.task_list,
                &icons.task_list,
            ),
            (Command::Quote, &labels.quote, &names.quote, &icons.quote),
            (
                Command::CodeBlock,
                &labels.code_block,
                &names.code_block,
                &icons.code_block,
            ),
        ];
        let mut changed = false;
        let mut pressed = false;
        let mut ids = Vec::with_capacity(buttons.len());
        let mut response = ui
            .horizontal(|ui| {
                for (cmd, label, name, icon) in buttons {
                    let hover = match shortcut(cmd) {
                        Some(s) => format!("{name} ({})", ui.ctx().format_shortcut(&s)),
                        None => name.clone(),
                    };
                    let response = button(ui, label, name, icon).on_hover_text(hover);
                    ids.push(response.id);
                    pressed |= response.is_pointer_button_down_on();
                    if response.clicked() {
                        changed |= run(ui, text, editor_id, cmd);
                        pressed = true;
                    }
                }
            })
            .response;
        let bar_focused = focused.is_some_and(|f| ids.contains(&f));
        ctx.data_mut(|d| {
            d.insert_temp(pressed_id(editor_id), pressed);
            d.insert_temp(buttons_id(editor_id), ids);
        });
        if !editor_focused && !pressed && !bar_focused {
            ctx.data_mut(|d| d.remove_temp::<CCursorRange>(held_id(editor_id)));
        }
        if changed {
            response.mark_changed();
        }
        response
    }
}

/// One button: the icon alone if there is one, else the label; named `name` for screen
/// readers either way.
fn button(ui: &mut Ui, label: &str, name: &str, icon: &Option<ImageSource<'static>>) -> Response {
    let response = match icon {
        None => ui.add(Button::new(label)),
        Some(source) => {
            let size = ui.spacing().interact_size.y - 2.0 * ui.spacing().button_padding.y;
            let image = Image::new(source.clone())
                .fit_to_exact_size(egui::vec2(size, size))
                .tint(ui.visuals().text_color());
            ui.add(Button::image(image))
        }
    };
    response.widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), name));
    response
}

/// Where the toolbar keeps whether a button was down last frame.
fn pressed_id(editor: Id) -> Id {
    editor.with("toolbar-pressed")
}

/// Where the toolbar keeps the ids of its buttons, as of last frame.
fn buttons_id(editor: Id) -> Id {
    editor.with("toolbar-buttons")
}

/// Where the toolbar keeps the editor's selection (in chars) while a button is held or
/// focused.
fn held_id(editor: Id) -> Id {
    editor.with("toolbar-held")
}

/// Where the toolbar keeps the selection to give the editor back next frame.
fn restore_id(editor: Id) -> Id {
    editor.with("toolbar-restore")
}

/// The selection a button acts on: the editor's, or what it was when the button was
/// pressed or focused; the end of the text if it never had one.
fn target(ctx: &Context, id: Id, text: &str) -> Range<usize> {
    let held = ctx
        .data(|d| d.get_temp::<CCursorRange>(held_id(id)))
        .map(|r| {
            let [a, b] = r.sorted_cursors();
            char_to_byte(text, a.index.0)..char_to_byte(text, b.index.0)
        });
    // The text may have changed since the selection was kept.
    clamp(
        text,
        held.or_else(|| selection(ctx, id, text))
            .unwrap_or(text.len()..text.len()),
    )
}

/// Apply `cmd` to the editor's selection as one undo step, and give the editor the focus
/// back. Whether the text changed.
fn run(ui: &Ui, text: &mut String, id: Id, cmd: Command) -> bool {
    let ctx = ui.ctx();
    let sel = target(ctx, id, text);
    let cmd = match cmd {
        Command::Heading(_) => Command::Heading(next_heading(text, sel.start)),
        cmd => cmd,
    };
    let edit = command(text, sel.clone(), cmd);
    if let Some(edit) = &edit {
        // So that undo comes back to this selection, not the collapsed one.
        set_selection(ctx, id, text, sel);
        apply(ctx, id, text, edit);
        // The editor may drop its selection this frame (see `show`).
        let now = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| d.insert_temp(restore_id(id), (now, edit.selection.clone())));
        mark_edited(ctx, id);
    }
    ctx.data_mut(|d| d.remove_temp::<CCursorRange>(held_id(id)));
    ui.memory_mut(|m| m.request_focus(id));
    ctx.request_repaint();
    edit.is_some()
}

/// The heading level the heading button sets next on the line holding byte `at`: none, 1,
/// 2, 3, none.
fn next_heading(text: &str, at: usize) -> u8 {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    match heading_level(&text[start..end]) {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 0,
    }
}
