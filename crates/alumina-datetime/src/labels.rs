//! The words the widgets show, and the settings every calendar shares.

use std::sync::{Arc, LazyLock};

use chrono::{Local, NaiveDate, Weekday};

use crate::{CalendarIcons, PeriodKind};

/// Every word the widgets show. [`Labels::default`] is English; an app passes its own
/// translation (corundum's Fluent strings, say) through [`CalendarConfig::labels`].
#[derive(Clone, Debug, PartialEq)]
pub struct Labels {
    /// January to December, as a heading reads them.
    pub months: [String; 12],
    /// January to December, short, for the month grid.
    pub months_short: [String; 12],
    /// Monday to Sunday, short, for the heading of the day grid.
    pub weekdays_short: [String; 7],
    /// The heading of the week-number column.
    pub week_column: String,
    /// The names of the kinds of period, for the switch between them.
    pub day: String,
    pub week: String,
    pub month: String,
    pub year: String,
    pub range: String,
    pub today: String,
    pub now: String,
    pub clear: String,
    pub previous: String,
    pub next: String,
    /// The tooltip of the heading that zooms out to the months / the years.
    pub choose_month: String,
    pub choose_year: String,
    pub am: String,
    pub pm: String,
    /// What a picker button reads with nothing picked.
    pub none: String,
    /// The text field's hint in the period picker.
    pub period_hint: String,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            months: [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ]
            .map(String::from),
            months_short: [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]
            .map(String::from),
            weekdays_short: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(String::from),
            week_column: "W".into(),
            day: "Day".into(),
            week: "Week".into(),
            month: "Month".into(),
            year: "Year".into(),
            range: "Range".into(),
            today: "Today".into(),
            now: "Now".into(),
            clear: "Clear".into(),
            previous: "Previous".into(),
            next: "Next".into(),
            choose_month: "Choose a month".into(),
            choose_year: "Choose a year".into(),
            am: "AM".into(),
            pm: "PM".into(),
            none: "—".into(),
            period_hint: "2026-11-03, 2026-W45, 2026-11, 2026 …".into(),
        }
    }
}

impl Labels {
    pub fn month(&self, month: u32) -> &str {
        &self.months[(month.clamp(1, 12) - 1) as usize]
    }

    pub fn month_short(&self, month: u32) -> &str {
        &self.months_short[(month.clamp(1, 12) - 1) as usize]
    }

    pub fn weekday_short(&self, day: Weekday) -> &str {
        &self.weekdays_short[day.num_days_from_monday() as usize]
    }

    pub fn kind(&self, kind: PeriodKind) -> &str {
        match kind {
            PeriodKind::Day => &self.day,
            PeriodKind::Week => &self.week,
            PeriodKind::Month => &self.month,
            PeriodKind::Year => &self.year,
            PeriodKind::Range => &self.range,
        }
    }
}

static ENGLISH: LazyLock<Arc<Labels>> = LazyLock::new(|| Arc::new(Labels::default()));

/// What every calendar shares: the words, the first day of the week, today.
#[derive(Clone, Debug)]
pub struct CalendarConfig {
    pub labels: Arc<Labels>,
    /// The app's icons for the buttons; none by default (arrows drawn, words for the rest).
    pub icons: Arc<CalendarIcons>,
    /// The day the weeks start on: the first column, and the weeks a week pick spans.
    pub first_weekday: Weekday,
    /// A column of ISO week numbers before the days (when weeks start on Monday).
    pub show_week_numbers: bool,
    /// The side of a day's cell, in points.
    pub cell_size: f32,
    /// Today, for its mark and the Today button; `None` reads the local clock.
    pub today: Option<NaiveDate>,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            labels: ENGLISH.clone(),
            icons: Arc::default(),
            first_weekday: Weekday::Mon,
            show_week_numbers: false,
            cell_size: 28.0,
            today: None,
        }
    }
}

impl CalendarConfig {
    pub fn labels(mut self, labels: impl Into<Arc<Labels>>) -> Self {
        self.labels = labels.into();
        self
    }

    /// The app's icons for the buttons that can be one; [`Labels`] stay their names.
    pub fn icons(mut self, icons: impl Into<Arc<CalendarIcons>>) -> Self {
        self.icons = icons.into();
        self
    }

    pub fn first_weekday(mut self, day: Weekday) -> Self {
        self.first_weekday = day;
        self
    }

    pub fn show_week_numbers(mut self, show: bool) -> Self {
        self.show_week_numbers = show;
        self
    }

    pub fn cell_size(mut self, size: f32) -> Self {
        self.cell_size = size;
        self
    }

    pub fn today(mut self, today: NaiveDate) -> Self {
        self.today = Some(today);
        self
    }

    pub(crate) fn today_or_local(&self) -> NaiveDate {
        self.today.unwrap_or_else(|| Local::now().date_naive())
    }

    /// Whether the week-number column is drawn: ISO numbers only fit Monday weeks.
    pub(crate) fn week_numbers_drawn(&self) -> bool {
        self.show_week_numbers && self.first_weekday == Weekday::Mon
    }
}
