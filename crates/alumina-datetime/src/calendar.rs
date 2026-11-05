//! A calendar that picks a day, a week, a month, a year or a range of days.

use chrono::{Datelike as _, Days, NaiveDate};
use egui::{
    Align2, CursorIcon, FontId, Id, IdSalt, Rect, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    vec2,
};

use crate::calc::{add_months, add_years, month_start, week_start, weekdays_from};
use crate::icons::small_button;
use crate::{CalendarConfig, Period, PeriodKind};

/// What the calendar's grid shows: zooming out from the days to the months to the years.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CalendarView {
    /// One month's days, six weeks of them.
    Days,
    /// One year's twelve months.
    Months,
    /// A decade, with a year either side.
    Years,
}

impl CalendarView {
    /// The view a kind of period is picked in: a month in the months, a year in the years,
    /// the rest in the days.
    pub fn for_kind(kind: PeriodKind) -> Self {
        match kind {
            PeriodKind::Month => CalendarView::Months,
            PeriodKind::Year => CalendarView::Years,
            PeriodKind::Day | PeriodKind::Week | PeriodKind::Range => CalendarView::Days,
        }
    }

    fn depth(self) -> u8 {
        match self {
            CalendarView::Days => 0,
            CalendarView::Months => 1,
            CalendarView::Years => 2,
        }
    }
}

/// What a calendar keeps between frames: the month on show, the view, the kind of period
/// being picked, and the first click of a range. The caller keeps it (a popup keeps it in
/// egui's memory).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarState {
    /// A day in the month (the year, in the years view) on show.
    pub cursor: NaiveDate,
    pub view: CalendarView,
    pub kind: PeriodKind,
    /// The first end of a range picked so far.
    pub anchor: Option<NaiveDate>,
}

impl CalendarState {
    /// The days of `cursor`'s month, picking days.
    pub fn new(cursor: NaiveDate) -> Self {
        Self {
            cursor,
            view: CalendarView::Days,
            kind: PeriodKind::Day,
            anchor: None,
        }
    }

    /// Opened on `period` (on `today` without one), picking `period`'s kind when it is one
    /// of `kinds`, else the first of `kinds`.
    pub fn for_period(period: Option<&Period>, kinds: &[PeriodKind], today: NaiveDate) -> Self {
        let kind = period
            .map(Period::kind)
            .filter(|k| kinds.contains(k))
            .or_else(|| kinds.first().copied())
            .unwrap_or(PeriodKind::Day);
        Self {
            cursor: period.map_or(today, Period::first_day),
            view: CalendarView::for_kind(kind),
            kind,
            anchor: None,
        }
    }

    fn set_kind(&mut self, kind: PeriodKind) {
        self.kind = kind;
        self.view = CalendarView::for_kind(kind);
        self.anchor = None;
    }
}

/// What the user did to the calendar this frame.
#[derive(Debug)]
pub struct CalendarResponse {
    /// The whole calendar's area.
    pub response: egui::Response,
    /// The period clicked (or the second end of a range).
    pub picked: Option<Period>,
    /// The Clear button was clicked.
    pub cleared: bool,
}

/// The count of marks a day gets (entries on it, say); a dot is drawn above zero.
pub type DayBadge<'a> = &'a dyn Fn(NaiveDate) -> usize;

/// A month's days in a grid, which zooms out to the months and the years.
///
/// Which kinds of period it picks is the caller's ([`Calendar::kinds`]); with more than one, a
/// switch above the grid chooses. Clicking picks a period of that kind: a day, the week a
/// day is in, a month in the months view, a year in the years view, or, with two clicks, the
/// days between. The heading zooms out (days → months → years); a click in a coarser view
/// than the kind's zooms back in.
///
/// ```no_run
/// # use alumina_datetime::{Calendar, CalendarState, Period, PeriodKind};
/// # fn ui(ui: &mut egui::Ui, state: &mut CalendarState, period: &mut Option<Period>) {
/// let r = Calendar::new("filter", state)
///     .kinds(&PeriodKind::ALL)
///     .selection(*period)
///     .show(ui);
/// if let Some(p) = r.picked {
///     *period = Some(p);
/// }
/// # }
/// ```
pub struct Calendar<'a> {
    id_salt: IdSalt,
    state: &'a mut CalendarState,
    selection: Option<Period>,
    kinds: &'a [PeriodKind],
    config: CalendarConfig,
    day_badge: Option<DayBadge<'a>>,
    show_today: bool,
    show_clear: bool,
}

const GAP: f32 = 2.0;

impl<'a> Calendar<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, state: &'a mut CalendarState) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            state,
            selection: None,
            kinds: &[PeriodKind::Day],
            config: CalendarConfig::default(),
            day_badge: None,
            show_today: true,
            show_clear: false,
        }
    }

    /// The period to highlight (the value picked before).
    pub fn selection(mut self, selection: Option<Period>) -> Self {
        self.selection = selection;
        self
    }

    /// The kinds of period the user may pick, in the order the switch shows them. Only days by
    /// default.
    pub fn kinds(mut self, kinds: &'a [PeriodKind]) -> Self {
        self.kinds = kinds;
        self
    }

    pub fn config(mut self, config: CalendarConfig) -> Self {
        self.config = config;
        self
    }

    /// A dot on each day the closure gives a count above zero for (entries on that day, say).
    pub fn day_badge(mut self, badge: DayBadge<'a>) -> Self {
        self.day_badge = Some(badge);
        self
    }

    /// The Today button, which shows today and picks the period of the kind being picked
    /// that holds it. On by default.
    pub fn show_today(mut self, show: bool) -> Self {
        self.show_today = show;
        self
    }

    /// The Clear button ([`CalendarResponse::cleared`]). Off by default.
    pub fn show_clear(mut self, show: bool) -> Self {
        self.show_clear = show;
        self
    }

    pub fn show(self, ui: &mut Ui) -> CalendarResponse {
        let Calendar {
            id_salt,
            state,
            selection,
            kinds,
            config,
            day_badge,
            show_today,
            show_clear,
        } = self;
        let kinds: &[PeriodKind] = if kinds.is_empty() {
            &[PeriodKind::Day]
        } else {
            kinds
        };
        if !kinds.contains(&state.kind) {
            state.set_kind(kinds[0]);
        }
        // The view never goes finer than the kind's.
        if state.view.depth() < CalendarView::for_kind(state.kind).depth() {
            state.view = CalendarView::for_kind(state.kind);
        }
        if state.kind != PeriodKind::Range {
            state.anchor = None;
        }

        let id = ui.make_persistent_id(id_salt);
        let mut picked = None;
        let mut cleared = false;
        let today = config.today_or_local();
        let cols = 7 + usize::from(config.week_numbers_drawn());
        let cell = config.cell_size;
        let width = cols as f32 * cell + (cols - 1) as f32 * GAP;

        let inner = ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            if kinds.len() > 1 {
                ui.horizontal_wrapped(|ui| {
                    ui.set_max_width(width);
                    for &kind in kinds {
                        if ui
                            .selectable_label(state.kind == kind, config.labels.kind(kind))
                            .clicked()
                        {
                            state.set_kind(kind);
                        }
                    }
                });
            }

            heading(ui, state, &config, width);

            let mut ctx = GridCtx {
                id,
                state,
                selection,
                config: &config,
                day_badge,
                today,
                width,
                picked: &mut picked,
            };
            match ctx.state.view {
                CalendarView::Days => ctx.days(ui),
                CalendarView::Months => ctx.months(ui),
                CalendarView::Years => ctx.years(ui),
            }

            if show_today || show_clear {
                ui.allocate_ui_with_layout(
                    vec2(width, ui.spacing().interact_size.y),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if show_today
                            && small_button(ui, &config.labels.today, &config.icons.today).clicked()
                        {
                            state.cursor = today;
                            state.view = CalendarView::for_kind(state.kind);
                            state.anchor = None;
                            picked =
                                Some(Period::containing(state.kind, today, config.first_weekday));
                        }
                        if show_clear {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if small_button(ui, &config.labels.clear, &config.icons.clear)
                                        .clicked()
                                    {
                                        state.anchor = None;
                                        cleared = true;
                                    }
                                },
                            );
                        }
                    },
                );
            }
        });

        if state.anchor.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.anchor = None;
        }

        let mut response = inner.response;
        if picked.is_some() || cleared {
            response.mark_changed();
        }
        CalendarResponse {
            response,
            picked,
            cleared,
        }
    }
}

/// The arrows and the title above the grid; the title zooms out.
fn heading(ui: &mut Ui, state: &mut CalendarState, config: &CalendarConfig, width: f32) {
    let labels = &config.labels;
    let height = ui.spacing().interact_size.y;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let arrow = vec2(height, height);
    let prev = Rect::from_min_size(rect.min, arrow);
    let next = Rect::from_min_size(rect.right_top() - vec2(arrow.x, 0.0), arrow);
    let title = Rect::from_min_max(prev.right_top(), next.left_bottom());

    let step = |state: &mut CalendarState, sign: i32| {
        state.cursor = match state.view {
            CalendarView::Days => add_months(state.cursor, sign),
            CalendarView::Months => add_years(state.cursor, sign),
            CalendarView::Years => add_years(state.cursor, 10 * sign),
        };
    };
    if chevron(ui, prev, -1.0, &labels.previous, &config.icons.previous).clicked() {
        step(state, -1);
    }
    if chevron(ui, next, 1.0, &labels.next, &config.icons.next).clicked() {
        step(state, 1);
    }

    let year = state.cursor.year();
    let text = match state.view {
        CalendarView::Days => format!("{} {year}", labels.month(state.cursor.month())),
        CalendarView::Months => year.to_string(),
        CalendarView::Years => {
            let first = decade_start(year);
            format!("{first} – {}", first + 9)
        }
    };
    let text = egui::RichText::new(text).strong();
    match state.view {
        CalendarView::Days | CalendarView::Months => {
            let (up, tip) = if state.view == CalendarView::Days {
                (CalendarView::Months, &labels.choose_month)
            } else {
                (CalendarView::Years, &labels.choose_year)
            };
            let r = ui
                .put(title, egui::Button::new(text).frame(false))
                .on_hover_text(tip);
            if r.clicked() {
                state.view = up;
            }
        }
        CalendarView::Years => {
            ui.put(title, egui::Label::new(text));
        }
    }
}

/// An arrow button drawn with lines, needing no font: pointing left (`dir < 0`) or right;
/// the app's `icon` instead when it has one.
fn chevron(
    ui: &mut Ui,
    rect: Rect,
    dir: f32,
    label: &str,
    icon: &Option<egui::ImageSource<'static>>,
) -> egui::Response {
    let id = ui.next_auto_id();
    ui.skip_ahead_auto_ids(1);
    let resp = ui
        .interact(rect, id, Sense::click())
        .on_hover_text(label)
        .on_hover_cursor(CursorIcon::PointingHand);
    resp.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let visuals = ui.style().interact(&resp);
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect.shrink(2.0), 4.0, visuals.weak_bg_fill);
    }
    if let Some(source) = icon {
        // Fit inside the shrunk rect, keeping the aspect ratio; tinted like the other
        // icons (the static text colour).
        let side = rect.height() * 0.6;
        egui::Image::new(source.clone())
            .fit_to_exact_size(vec2(side, side))
            .tint(ui.visuals().text_color())
            .paint_at(ui, Rect::from_center_size(rect.center(), vec2(side, side)));
        return resp;
    }
    let c = rect.center();
    let h = (rect.height() * 0.18).max(3.0);
    let tip = c + vec2(h * 0.5 * dir, 0.0);
    let back = c - vec2(h * 0.5 * dir, 0.0);
    ui.painter().line(
        vec![back + vec2(0.0, -h), tip, back + vec2(0.0, h)],
        Stroke::new(1.5, visuals.fg_stroke.color),
    );
    resp
}

struct GridCtx<'s, 'a> {
    id: Id,
    state: &'s mut CalendarState,
    selection: Option<Period>,
    config: &'s CalendarConfig,
    day_badge: Option<DayBadge<'a>>,
    today: NaiveDate,
    width: f32,
    picked: &'s mut Option<Period>,
}

/// How a cell is drawn.
#[derive(Default)]
struct CellLook {
    in_month: bool,
    selected: bool,
    preview: bool,
    current: bool,
    hovered: bool,
    badge: bool,
}

fn paint_cell(ui: &Ui, rect: Rect, text: &str, look: &CellLook) {
    let v = ui.visuals();
    let painter = ui.painter();
    let radius = 4.0;
    let fill = if look.selected {
        Some(v.selection.bg_fill)
    } else if look.preview {
        Some(v.selection.bg_fill.gamma_multiply(0.45))
    } else if look.hovered {
        Some(v.widgets.hovered.weak_bg_fill)
    } else {
        None
    };
    if let Some(fill) = fill {
        painter.rect_filled(rect, radius, fill);
    }
    let text_color = if look.selected {
        v.selection.stroke.color
    } else if !look.in_month {
        v.weak_text_color()
    } else {
        v.text_color()
    };
    if look.current {
        let ring = if look.selected {
            v.selection.stroke.color
        } else {
            v.selection.bg_fill
        };
        painter.rect_stroke(rect, radius, Stroke::new(1.5, ring), StrokeKind::Inside);
    }
    let font = FontId::proportional((rect.height() * 0.45).clamp(10.0, 16.0));
    painter.text(rect.center(), Align2::CENTER_CENTER, text, font, text_color);
    if look.badge {
        let center = rect.right_bottom() - vec2(5.0, 5.0);
        painter.circle_filled(center, 2.5, text_color.gamma_multiply(0.8));
    }
}

/// The first year of `year`'s decade (2020 for 2026).
fn decade_start(year: i32) -> i32 {
    year - year.rem_euclid(10)
}

fn overlaps(p: Option<Period>, first: NaiveDate, last: NaiveDate) -> bool {
    p.is_some_and(|p| p.first_day() <= last && first <= p.last_day())
}

impl GridCtx<'_, '_> {
    /// The grid's height, the same in every view so a popup does not jump when zooming.
    fn body_height(&self) -> f32 {
        let cell = self.config.cell_size;
        let heading_row = (cell * 0.75).round();
        heading_row + GAP + 6.0 * cell + 5.0 * GAP
    }

    fn days(&mut self, ui: &mut Ui) {
        let cfg = self.config;
        let cell = cfg.cell_size;
        let week_col = cfg.week_numbers_drawn();
        let heading_h = (cell * 0.75).round();
        let (rect, _) =
            ui.allocate_exact_size(vec2(self.width, self.body_height()), Sense::hover());
        let col_x = |c: usize| rect.left() + c as f32 * (cell + GAP);
        let row_y = |r: usize| rect.top() + heading_h + GAP + r as f32 * (cell + GAP);
        let first_col = usize::from(week_col);

        // Weekday heading.
        let painter = ui.painter();
        let weak = ui.visuals().weak_text_color();
        let font = FontId::proportional((cell * 0.4).clamp(9.0, 13.0));
        if week_col {
            let r = Rect::from_min_size(egui::pos2(col_x(0), rect.top()), vec2(cell, heading_h));
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                &cfg.labels.week_column,
                font.clone(),
                weak,
            );
        }
        for (i, wd) in weekdays_from(cfg.first_weekday).into_iter().enumerate() {
            let r = Rect::from_min_size(
                egui::pos2(col_x(first_col + i), rect.top()),
                vec2(cell, heading_h),
            );
            painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                cfg.labels.weekday_short(wd),
                font.clone(),
                weak,
            );
        }

        let grid_start = week_start(month_start(self.state.cursor), cfg.first_weekday);
        let date_at = |r: usize, c: usize| grid_start + Days::new((r * 7 + c) as u64);

        // The day under the pointer, for the hover preview of a week or a range.
        let hover_pos = ui
            .ctx()
            .pointer_hover_pos()
            .filter(|p| rect.contains(*p) && ui.ui_contains_pointer());
        let hovered_date = hover_pos.and_then(|p| {
            let c = ((p.x - rect.left()) / (cell + GAP)).floor() as isize - first_col as isize;
            let r = ((p.y - row_y(0)) / (cell + GAP)).floor() as isize;
            ((0..7).contains(&c) && (0..6).contains(&r)).then(|| date_at(r as usize, c as usize))
        });
        let preview = match (self.state.kind, hovered_date) {
            (PeriodKind::Week, Some(d)) => Some(Period::week_of(d, cfg.first_weekday)),
            (PeriodKind::Range, Some(d)) => self.state.anchor.map(|a| Period::range(a, d)),
            (PeriodKind::Range, None) => self.state.anchor.map(|a| Period::range(a, a)),
            _ => None,
        };

        for r in 0..6 {
            let week_first = date_at(r, 0);
            if week_col {
                let cell_rect =
                    Rect::from_min_size(egui::pos2(col_x(0), row_y(r)), vec2(cell, cell));
                let resp = ui.interact(cell_rect, self.id.with(("week", r)), Sense::click());
                let week = Period::Week { start: week_first };
                resp.widget_info(|| {
                    WidgetInfo::selected(
                        Role::Button,
                        self.state.kind == PeriodKind::Week,
                        self.selection == Some(week),
                        week.to_string(),
                    )
                });
                let can_pick = self.state.kind == PeriodKind::Week;
                if can_pick {
                    resp.clone().on_hover_cursor(CursorIcon::PointingHand);
                }
                let look = CellLook {
                    in_month: false,
                    selected: self.selection == Some(week),
                    hovered: can_pick && resp.hovered(),
                    ..Default::default()
                };
                let text = format!("{:02}", week_first.iso_week().week());
                paint_cell(ui, cell_rect, &text, &look);
                if can_pick && resp.clicked() {
                    *self.picked = Some(week);
                }
            }
            for c in 0..7 {
                let date = date_at(r, c);
                let cell_rect = Rect::from_min_size(
                    egui::pos2(col_x(first_col + c), row_y(r)),
                    vec2(cell, cell),
                );
                let resp = ui
                    .interact(cell_rect, self.id.with(("day", r, c)), Sense::click())
                    .on_hover_cursor(CursorIcon::PointingHand);
                let selected = self.selection.is_some_and(|p| p.contains(date));
                resp.widget_info(|| {
                    WidgetInfo::selected(Role::Button, true, selected, date.format("%Y-%m-%d"))
                });
                let look = CellLook {
                    in_month: date.month() == self.state.cursor.month(),
                    selected: self.selection.is_some_and(|p| p.contains(date))
                        || self.state.anchor == Some(date),
                    preview: preview.is_some_and(|p| p.contains(date)),
                    current: date == self.today,
                    hovered: resp.hovered(),
                    badge: self.day_badge.is_some_and(|b| b(date) > 0),
                };
                paint_cell(ui, cell_rect, &date.day().to_string(), &look);
                if resp.clicked() {
                    self.click_day(date);
                }
            }
        }
    }

    fn click_day(&mut self, date: NaiveDate) {
        let first = self.config.first_weekday;
        *self.picked = match self.state.kind {
            PeriodKind::Range => match self.state.anchor.take() {
                Some(a) => Some(Period::range(a, date)),
                None => {
                    self.state.anchor = Some(date);
                    None
                }
            },
            kind => Some(Period::containing(kind, date, first)),
        };
    }

    /// A 3 × 4 grid of `cells` the height of the days view; `click` gets the index clicked.
    fn twelve(
        &mut self,
        ui: &mut Ui,
        salt: &str,
        label: impl Fn(usize) -> String,
        name: impl Fn(usize) -> String,
        look: impl Fn(usize) -> CellLook,
    ) -> Option<usize> {
        let (rect, _) =
            ui.allocate_exact_size(vec2(self.width, self.body_height()), Sense::hover());
        let w = (rect.width() - 2.0 * GAP) / 3.0;
        let h = (rect.height() - 3.0 * GAP) / 4.0;
        let mut clicked = None;
        for i in 0..12 {
            let (r, c) = (i / 3, i % 3);
            let cell_rect = Rect::from_min_size(
                rect.min + vec2(c as f32 * (w + GAP), r as f32 * (h + GAP)),
                vec2(w, h),
            );
            let resp = ui
                .interact(cell_rect, self.id.with((salt, i)), Sense::click())
                .on_hover_cursor(CursorIcon::PointingHand);
            let mut l = look(i);
            resp.widget_info(|| WidgetInfo::selected(Role::Button, true, l.selected, name(i)));
            l.hovered = resp.hovered();
            paint_cell(ui, cell_rect.shrink(4.0), &label(i), &l);
            if resp.clicked() {
                clicked = Some(i);
            }
        }
        clicked
    }

    fn months(&mut self, ui: &mut Ui) {
        let year = self.state.cursor.year();
        let labels = self.config.labels.clone();
        let today = self.today;
        let selection = self.selection;
        let month = |i: usize| Period::Month {
            year,
            month: i as u32 + 1,
        };
        let clicked = self.twelve(
            ui,
            "month",
            |i| labels.month_short(i as u32 + 1).to_string(),
            |i| month(i).to_string(),
            |i| {
                let m = month(i);
                CellLook {
                    in_month: true,
                    selected: overlaps(selection, m.first_day(), m.last_day()),
                    current: m.contains(today),
                    ..Default::default()
                }
            },
        );
        if let Some(i) = clicked {
            let m = month(i);
            if self.state.kind == PeriodKind::Month {
                *self.picked = Some(m);
            } else {
                let day = self.state.cursor.day().min(m.last_day().day());
                self.state.cursor = m.first_day().with_day(day).unwrap_or(m.first_day());
                self.state.view = CalendarView::Days;
            }
        }
    }

    fn years(&mut self, ui: &mut Ui) {
        let cursor = self.state.cursor.year();
        // The decade, with a year either side.
        let first = decade_start(cursor) - 1;
        let today = self.today;
        let selection = self.selection;
        let clicked = self.twelve(
            ui,
            "year",
            |i| (first + i as i32).to_string(),
            |i| (first + i as i32).to_string(),
            |i| {
                let y = Period::Year(first + i as i32);
                CellLook {
                    in_month: (1..=10).contains(&i),
                    selected: overlaps(selection, y.first_day(), y.last_day()),
                    current: y.contains(today),
                    ..Default::default()
                }
            },
        );
        if let Some(i) = clicked {
            let year = first + i as i32;
            if self.state.kind == PeriodKind::Year {
                *self.picked = Some(Period::Year(year));
            } else {
                self.state.cursor = add_years(self.state.cursor, year - cursor);
                self.state.view = CalendarView::Months;
            }
        }
    }
}
