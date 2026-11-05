//! Buttons that open a calendar or drums in a popup, and the calendar beside drums.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike as _};
use egui::{Id, IdSalt, Popup, PopupCloseBehavior, Response, Ui, Widget};

use crate::icons::small_button;
use crate::{Calendar, CalendarConfig, CalendarState, DayBadge, Period, PeriodKind, TimeDrum};

const DAY_FMT: &str = "%Y-%m-%d";

/// A text field over a value: it shows the value, and while the user types, each text that
/// reads as a value sets it (red until one does). Returns the value typed.
fn value_field<T>(
    ui: &mut Ui,
    id: Id,
    value: &T,
    show: impl Fn(&T) -> String,
    read: impl Fn(&str) -> Option<T>,
    hint: &str,
) -> Option<T> {
    let mut text = ui
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| show(value));
    let valid = read(&text).is_some();
    let mut edit = egui::TextEdit::singleline(&mut text)
        .id(id.with("edit"))
        .hint_text(hint)
        .desired_width(f32::INFINITY);
    if !valid {
        edit = edit.text_color(ui.visuals().error_fg_color);
    }
    let r = ui.add(edit);
    let out = if r.changed() { read(&text) } else { None };
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text));
    } else {
        // Not being typed in: follow the value.
        ui.data_mut(|d| d.remove::<String>(id));
    }
    out
}

/// The calendar's state in egui's memory, under the popup's id; set afresh from `seed` when
/// `reset`.
fn popup_state(
    ui: &Ui,
    id: Id,
    reset: bool,
    seed: impl FnOnce() -> CalendarState,
) -> CalendarState {
    let key = id.with("calendar");
    if reset {
        return seed();
    }
    ui.data(|d| d.get_temp(key)).unwrap_or_else(seed)
}

fn store_state(ui: &Ui, id: Id, state: CalendarState) {
    ui.data_mut(|d| d.insert_temp(id.with("calendar"), state));
}

fn popup(button: &Response, id: Id, add: impl FnOnce(&mut Ui) -> bool) {
    Popup::from_toggle_button_response(button)
        .id(id)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if add(ui) {
                Popup::close_id(ui.ctx(), id);
            }
        });
}

/// A button reading a date (or a dash); it opens a calendar and a text field to pick one.
pub struct DatePickerButton<'a> {
    id_salt: IdSalt,
    date: &'a mut Option<NaiveDate>,
    config: CalendarConfig,
    format: String,
    clearable: bool,
    day_badge: Option<DayBadge<'a>>,
}

impl<'a> DatePickerButton<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, date: &'a mut Option<NaiveDate>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            date,
            config: CalendarConfig::default(),
            format: DAY_FMT.into(),
            clearable: true,
            day_badge: None,
        }
    }

    pub fn config(mut self, config: CalendarConfig) -> Self {
        self.config = config;
        self
    }

    /// How the button writes the date (chrono's `strftime`); `%Y-%m-%d` by default. The
    /// text field always reads and writes `%Y-%m-%d`.
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = format.into();
        self
    }

    /// Whether the date can be cleared (a Clear button, an empty field). On by default.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    pub fn day_badge(mut self, badge: DayBadge<'a>) -> Self {
        self.day_badge = Some(badge);
        self
    }
}

impl Widget for DatePickerButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let DatePickerButton {
            id_salt,
            date,
            config,
            format,
            clearable,
            day_badge,
        } = self;
        let text = date.map_or_else(
            || config.labels.none.clone(),
            |d| d.format(&format).to_string(),
        );
        let mut button = ui.button(text);
        let id = ui.make_persistent_id(id_salt);
        let today = config.today_or_local();
        let opening = button.clicked();
        let mut changed = false;

        popup(&button, id, |ui| {
            let typed = value_field(
                ui,
                id.with("field"),
                &*date,
                |d| d.map(|d| d.format(DAY_FMT).to_string()).unwrap_or_default(),
                |s| match s.trim() {
                    "" if clearable => Some(None),
                    s => NaiveDate::parse_from_str(s, DAY_FMT).ok().map(Some),
                },
                "YYYY-MM-DD",
            );
            let mut state = popup_state(ui, id, opening, || {
                CalendarState::new(date.unwrap_or(today))
            });
            if let Some(typed) = typed {
                *date = typed;
                if let Some(d) = typed {
                    state.cursor = d;
                }
                changed = true;
            }
            let mut cal = Calendar::new(id.with("cal"), &mut state)
                .selection(date.map(Period::Day))
                .config(config.clone())
                .show_clear(clearable);
            if let Some(b) = day_badge {
                cal = cal.day_badge(b);
            }
            let r = cal.show(ui);
            store_state(ui, id, state);
            let mut close = false;
            if let Some(Period::Day(d)) = r.picked {
                *date = Some(d);
                changed = true;
                close = true;
            }
            if r.cleared {
                *date = None;
                changed = true;
                close = true;
            }
            close
        });
        if changed {
            button.mark_changed();
        }
        button
    }
}

/// A button reading a period (or a dash); it opens a calendar to pick a day, a week, a month,
/// a year or a range (the kinds the caller allows), and a text field taking a period as
/// [`Period`] writes it.
pub struct PeriodPickerButton<'a> {
    id_salt: IdSalt,
    period: &'a mut Option<Period>,
    kinds: &'a [PeriodKind],
    config: CalendarConfig,
    clearable: bool,
    day_badge: Option<DayBadge<'a>>,
}

impl<'a> PeriodPickerButton<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, period: &'a mut Option<Period>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            period,
            kinds: &PeriodKind::ALL,
            config: CalendarConfig::default().show_week_numbers(true),
            clearable: true,
            day_badge: None,
        }
    }

    /// The kinds of period offered; all of them by default.
    pub fn kinds(mut self, kinds: &'a [PeriodKind]) -> Self {
        self.kinds = kinds;
        self
    }

    pub fn config(mut self, config: CalendarConfig) -> Self {
        self.config = config;
        self
    }

    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    pub fn day_badge(mut self, badge: DayBadge<'a>) -> Self {
        self.day_badge = Some(badge);
        self
    }
}

impl Widget for PeriodPickerButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let PeriodPickerButton {
            id_salt,
            period,
            kinds,
            config,
            clearable,
            day_badge,
        } = self;
        let text = period.map_or_else(|| config.labels.none.clone(), |p| p.to_string());
        let mut button = ui.button(text);
        if let Some(p) = *period
            && p.len_days() > 1
        {
            button = button.on_hover_text(format!(
                "{} – {}",
                p.first_day().format(DAY_FMT),
                p.last_day().format(DAY_FMT)
            ));
        }
        let id = ui.make_persistent_id(id_salt);
        let today = config.today_or_local();
        let opening = button.clicked();
        let mut changed = false;

        popup(&button, id, |ui| {
            let typed = value_field(
                ui,
                id.with("field"),
                &*period,
                |p| p.map(|p| p.to_string()).unwrap_or_default(),
                |s| match s.trim() {
                    "" if clearable => Some(None),
                    s => s
                        .parse::<Period>()
                        .ok()
                        .filter(|p| kinds.contains(&p.kind()))
                        .map(Some),
                },
                &config.labels.period_hint,
            );
            let mut state = popup_state(ui, id, opening, || {
                CalendarState::for_period(period.as_ref(), kinds, today)
            });
            if let Some(typed) = typed {
                *period = typed;
                if let Some(p) = typed {
                    state = CalendarState::for_period(Some(&p), kinds, today);
                }
                changed = true;
            }
            let mut cal = Calendar::new(id.with("cal"), &mut state)
                .kinds(kinds)
                .selection(*period)
                .config(config.clone())
                .show_clear(clearable);
            if let Some(b) = day_badge {
                cal = cal.day_badge(b);
            }
            let r = cal.show(ui);
            store_state(ui, id, state);
            if let Some(p) = r.picked {
                *period = Some(p);
                changed = true;
            }
            if r.cleared {
                *period = None;
                changed = true;
            }
            r.picked.is_some() || r.cleared
        });
        if changed {
            button.mark_changed();
        }
        button
    }
}

/// How times are shown on the drums and written on the buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeFormat {
    pub seconds: bool,
    pub hour12: bool,
    /// Minutes in steps of this on the drum.
    pub minute_step: usize,
}

impl Default for TimeFormat {
    fn default() -> Self {
        Self {
            seconds: false,
            hour12: false,
            minute_step: 1,
        }
    }
}

impl TimeFormat {
    /// The `strftime` pattern for a time.
    pub fn pattern(&self) -> &'static str {
        match (self.hour12, self.seconds) {
            (false, false) => "%H:%M",
            (false, true) => "%H:%M:%S",
            (true, false) => "%I:%M %p",
            (true, true) => "%I:%M:%S %p",
        }
    }

    fn drum<'a>(
        &self,
        id: impl egui::AsIdSalt,
        time: &'a mut NaiveTime,
        config: &CalendarConfig,
    ) -> TimeDrum<'a> {
        TimeDrum::new(id, time)
            .seconds(self.seconds)
            .hour12(self.hour12)
            .minute_step(self.minute_step)
            .labels(config.labels.clone())
    }
}

fn parse_time(s: &str) -> Option<NaiveTime> {
    let s = s.trim();
    ["%H:%M:%S", "%H:%M", "%I:%M:%S %p", "%I:%M %p"]
        .iter()
        .find_map(|f| NaiveTime::parse_from_str(s, f).ok())
}

fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
    ]
    .iter()
    .find_map(|f| NaiveDateTime::parse_from_str(s, f).ok())
}

/// A button reading a time; it opens drums and a text field to set it.
pub struct TimePickerButton<'a> {
    id_salt: IdSalt,
    time: &'a mut NaiveTime,
    format: TimeFormat,
    config: CalendarConfig,
}

impl<'a> TimePickerButton<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, time: &'a mut NaiveTime) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            time,
            format: TimeFormat::default(),
            config: CalendarConfig::default(),
        }
    }

    pub fn format(mut self, format: TimeFormat) -> Self {
        self.format = format;
        self
    }

    /// The words (AM / PM, Now).
    pub fn config(mut self, config: CalendarConfig) -> Self {
        self.config = config;
        self
    }
}

impl Widget for TimePickerButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let TimePickerButton {
            id_salt,
            time,
            format,
            config,
        } = self;
        let mut button = ui.button(time.format(format.pattern()).to_string());
        let id = ui.make_persistent_id(id_salt);
        let mut changed = false;
        popup(&button, id, |ui| {
            if let Some(t) = value_field(
                ui,
                id.with("field"),
                &*time,
                |t| t.format(format.pattern()).to_string(),
                parse_time,
                format.pattern(),
            ) {
                *time = t;
                changed = true;
            }
            changed |= ui
                .add(format.drum(id.with("drum"), time, &config))
                .changed();
            if small_button(ui, &config.labels.now, &config.icons.now).clicked() {
                let now = chrono::Local::now().time();
                *time = now.with_nanosecond(0).unwrap_or(now);
                changed = true;
            }
            false
        });
        if changed {
            button.mark_changed();
        }
        button
    }
}

/// A calendar beside time drums, picking a timestamp: a day on the calendar keeps the time,
/// the drums keep the day.
pub struct DateTimePicker<'a> {
    id_salt: IdSalt,
    datetime: &'a mut NaiveDateTime,
    state: &'a mut CalendarState,
    format: TimeFormat,
    config: CalendarConfig,
}

impl<'a> DateTimePicker<'a> {
    pub fn new(
        id_salt: impl egui::AsIdSalt,
        datetime: &'a mut NaiveDateTime,
        state: &'a mut CalendarState,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            datetime,
            state,
            format: TimeFormat::default(),
            config: CalendarConfig::default(),
        }
    }

    pub fn format(mut self, format: TimeFormat) -> Self {
        self.format = format;
        self
    }

    pub fn config(mut self, config: CalendarConfig) -> Self {
        self.config = config;
        self
    }
}

impl Widget for DateTimePicker<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let DateTimePicker {
            id_salt,
            datetime,
            state,
            format,
            config,
        } = self;
        let mut changed = false;
        let mut body = |ui: &mut Ui, side_by_side: bool| {
            let r = Calendar::new((id_salt, "cal"), state)
                .selection(Some(Period::Day(datetime.date())))
                .config(config.clone())
                .show(ui);
            if let Some(Period::Day(d)) = r.picked {
                *datetime = d.and_time(datetime.time());
                changed = true;
            }
            ui.separator();
            ui.vertical(|ui| {
                let mut time = datetime.time();
                if side_by_side {
                    ui.add_space(config.cell_size);
                }
                if ui
                    .add(format.drum((id_salt, "drum"), &mut time, &config))
                    .changed()
                {
                    *datetime = datetime.date().and_time(time);
                    changed = true;
                }
            });
        };
        // Side by side where they fit; on a phone's narrow screen the drums go under the
        // calendar.
        let inner = if ui.available_width() >= side_by_side_width(&config, &format) {
            ui.horizontal_top(|ui| body(ui, true))
        } else {
            ui.vertical(|ui| body(ui, false))
        };
        let mut response = inner.response;
        if changed {
            response.mark_changed();
        }
        response
    }
}

/// About the width of a calendar beside time drums.
fn side_by_side_width(config: &CalendarConfig, format: &TimeFormat) -> f32 {
    let cols = 7.0 + f32::from(u8::from(config.show_week_numbers));
    let calendar = cols * (config.cell_size + 2.0);
    let drums = 2 + usize::from(format.seconds) + usize::from(format.hour12);
    calendar + drums as f32 * 54.0 + 24.0
}

/// A button reading a timestamp; it opens [`DateTimePicker`] and a text field to set it.
pub struct DateTimePickerButton<'a> {
    id_salt: IdSalt,
    datetime: &'a mut NaiveDateTime,
    format: TimeFormat,
    config: CalendarConfig,
}

impl<'a> DateTimePickerButton<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, datetime: &'a mut NaiveDateTime) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            datetime,
            format: TimeFormat::default(),
            config: CalendarConfig::default(),
        }
    }

    pub fn format(mut self, format: TimeFormat) -> Self {
        self.format = format;
        self
    }

    pub fn config(mut self, config: CalendarConfig) -> Self {
        self.config = config;
        self
    }
}

impl Widget for DateTimePickerButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let DateTimePickerButton {
            id_salt,
            datetime,
            format,
            config,
        } = self;
        let pattern = format!("{DAY_FMT} {}", format.pattern());
        let mut button = ui.button(datetime.format(&pattern).to_string());
        let id = ui.make_persistent_id(id_salt);
        let opening = button.clicked();
        let mut changed = false;
        popup(&button, id, |ui| {
            let field_fmt = if format.seconds {
                "%Y-%m-%d %H:%M:%S"
            } else {
                "%Y-%m-%d %H:%M"
            };
            let mut state = popup_state(ui, id, opening, || CalendarState::new(datetime.date()));
            if let Some(dt) = value_field(
                ui,
                id.with("field"),
                &*datetime,
                |dt| dt.format(field_fmt).to_string(),
                parse_datetime,
                "YYYY-MM-DD HH:MM",
            ) {
                *datetime = dt;
                state.cursor = dt.date();
                changed = true;
            }
            changed |= ui
                .add(
                    DateTimePicker::new(id.with("picker"), datetime, &mut state)
                        .format(format)
                        .config(config.clone()),
                )
                .changed();
            if small_button(ui, &config.labels.now, &config.icons.now).clicked() {
                let now = chrono::Local::now().naive_local();
                *datetime = now.with_nanosecond(0).unwrap_or(now);
                state.cursor = datetime.date();
                changed = true;
            }
            store_state(ui, id, state);
            false
        });
        if changed {
            button.mark_changed();
        }
        button
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_parse() {
        let t = |h, m, s| NaiveTime::from_hms_opt(h, m, s).unwrap();
        assert_eq!(parse_time("09:30"), Some(t(9, 30, 0)));
        assert_eq!(parse_time(" 21:05:07 "), Some(t(21, 5, 7)));
        assert_eq!(parse_time("09:30 PM"), Some(t(21, 30, 0)));
        assert_eq!(parse_time("25:00"), None);
        for f in [
            TimeFormat::default(),
            TimeFormat {
                seconds: true,
                hour12: true,
                minute_step: 5,
            },
        ] {
            let x = t(13, 45, 12);
            let shown = x.format(f.pattern()).to_string();
            let back = parse_time(&shown).unwrap();
            assert_eq!(back.format(f.pattern()).to_string(), shown);
        }
    }

    #[test]
    fn datetimes_parse() {
        let dt = NaiveDate::from_ymd_opt(2026, 11, 3)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap();
        assert_eq!(parse_datetime("2026-11-03 08:00"), Some(dt));
        assert_eq!(parse_datetime("2026-11-03T08:00:00"), Some(dt));
        assert_eq!(parse_datetime("2026-11-03"), None);
    }
}
