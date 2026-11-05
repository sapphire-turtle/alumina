//! The page for alumina-datetime: every widget, with the settings they share.

use std::sync::Arc;

use alumina_datetime::chrono::{
    Datelike as _, Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday,
};
use alumina_datetime::{
    Calendar, CalendarConfig, CalendarState, DatePickerButton, DateTimePicker,
    DateTimePickerButton, Drum, Labels, Period, PeriodKind, PeriodPickerButton, TimeDrum,
    TimeFormat, TimePickerButton,
};
use corundum_framework_gui::egui;

pub struct DateTimeDemo {
    // Shared settings.
    japanese: bool,
    first_weekday: Weekday,
    week_numbers: bool,
    cell_size: f32,
    badges: bool,
    time_format: TimeFormat,

    // The inline calendar.
    kinds: Vec<PeriodKind>,
    calendar: CalendarState,
    calendar_value: Option<Period>,
    calendar_log: Vec<String>,

    // Drums.
    drum_value: i32,
    time: NaiveTime,

    // The calendar beside drums.
    datetime: NaiveDateTime,
    datetime_state: CalendarState,

    // Buttons.
    date: Option<NaiveDate>,
    period: Option<Period>,
    button_time: NaiveTime,
    button_datetime: NaiveDateTime,
}

impl Default for DateTimeDemo {
    fn default() -> Self {
        let now = Local::now().naive_local();
        let now = now
            .date()
            .and_hms_opt(now.time().hour(), now.time().minute(), 0)
            .unwrap_or(now);
        Self {
            japanese: false,
            first_weekday: Weekday::Mon,
            week_numbers: true,
            cell_size: 28.0,
            badges: true,
            time_format: TimeFormat::default(),
            kinds: PeriodKind::ALL.to_vec(),
            calendar: CalendarState::new(now.date()),
            calendar_value: None,
            calendar_log: Vec::new(),
            drum_value: 3,
            time: now.time(),
            datetime: now,
            datetime_state: CalendarState::new(now.date()),
            date: Some(now.date()),
            period: Some(Period::month_of(now.date())),
            button_time: now.time(),
            button_datetime: now,
        }
    }
}

use alumina_datetime::chrono::Timelike as _;

/// The words in Japanese, as an app would hand over its translation.
fn japanese() -> Labels {
    let mut l = Labels::default();
    l.months = std::array::from_fn(|i| format!("{}月", i + 1));
    l.months_short = std::array::from_fn(|i| format!("{}月", i + 1));
    l.weekdays_short = ["月", "火", "水", "木", "金", "土", "日"].map(String::from);
    l.week_column = "週".into();
    l.day = "日".into();
    l.week = "週".into();
    l.month = "月".into();
    l.year = "年".into();
    l.range = "期間".into();
    l.today = "今日".into();
    l.now = "現在".into();
    l.clear = "クリア".into();
    l.previous = "前へ".into();
    l.next = "次へ".into();
    l.choose_month = "月を選ぶ".into();
    l.choose_year = "年を選ぶ".into();
    l.am = "午前".into();
    l.pm = "午後".into();
    l.none = "未設定".into();
    l.period_hint = "2026-11-03、2026-W45、2026-11、2026 …".into();
    l
}

/// A made-up count of entries on a day, for the badges.
fn fake_entries(d: NaiveDate) -> usize {
    usize::from(d.ordinal().is_multiple_of(3) || d.day() == 1)
}

impl DateTimeDemo {
    fn config(&self) -> CalendarConfig {
        let mut config = CalendarConfig::default()
            .first_weekday(self.first_weekday)
            .show_week_numbers(self.week_numbers)
            .cell_size(self.cell_size);
        if self.japanese {
            config = config.labels(Arc::new(japanese()));
        }
        config
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let narrow = ui.available_width() < crate::NARROW;
        if !narrow {
            egui::Panel::left("datetime_settings")
                .resizable(false)
                .show(ui, |ui| {
                    ui.heading("Settings");
                    self.settings(ui);
                });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if narrow {
                    // On a phone the settings fold away above the widgets.
                    egui::CollapsingHeader::new("Settings")
                        .id_salt("datetime_settings_narrow")
                        .show(ui, |ui| self.settings(ui));
                    ui.separator();
                }
                self.buttons(ui);
                ui.separator();
                if narrow {
                    self.calendar(ui);
                    ui.separator();
                    self.drums(ui);
                } else {
                    ui.horizontal_top(|ui| {
                        self.calendar(ui);
                        ui.separator();
                        self.drums(ui);
                    });
                }
                ui.separator();
                self.datetime(ui);
            });
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.japanese, "日本語の表示");
        ui.horizontal(|ui| {
            ui.label("Week starts on");
            ui.selectable_value(&mut self.first_weekday, Weekday::Mon, "Mon");
            ui.selectable_value(&mut self.first_weekday, Weekday::Sun, "Sun");
        });
        ui.checkbox(&mut self.week_numbers, "Week numbers (Monday weeks)");
        ui.checkbox(&mut self.badges, "Entry dots");
        ui.add(egui::Slider::new(&mut self.cell_size, 20.0..=44.0).text("Cell size"));
        ui.separator();
        ui.label("Time");
        ui.checkbox(&mut self.time_format.seconds, "Seconds");
        ui.checkbox(&mut self.time_format.hour12, "12-hour");
        ui.horizontal(|ui| {
            ui.label("Minute step");
            for step in [1, 5, 15] {
                ui.selectable_value(&mut self.time_format.minute_step, step, step.to_string());
            }
        });
        ui.separator();
        ui.label("Calendar kinds");
        for kind in PeriodKind::ALL {
            let mut on = self.kinds.contains(&kind);
            if ui.checkbox(&mut on, format!("{kind:?}")).changed() {
                if on {
                    self.kinds.push(kind);
                    self.kinds
                        .sort_by_key(|k| PeriodKind::ALL.iter().position(|x| x == k));
                } else if self.kinds.len() > 1 {
                    self.kinds.retain(|k| *k != kind);
                }
            }
        }
    }

    fn buttons(&mut self, ui: &mut egui::Ui) {
        ui.heading("Picker buttons");
        let config = self.config();
        let badge: &dyn Fn(NaiveDate) -> usize = &fake_entries;
        egui::Grid::new("buttons")
            .num_columns(3)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Date");
                let mut b = DatePickerButton::new("date", &mut self.date).config(config.clone());
                if self.badges {
                    b = b.day_badge(badge);
                }
                ui.add(b);
                ui.monospace(format!("{:?}", self.date));
                ui.end_row();

                ui.label("Period");
                let mut b = PeriodPickerButton::new("period", &mut self.period)
                    .kinds(&self.kinds)
                    .config(config.clone());
                if self.badges {
                    b = b.day_badge(badge);
                }
                ui.add(b);
                ui.monospace(format!("{:?}", self.period));
                ui.end_row();

                ui.label("Time");
                ui.add(
                    TimePickerButton::new("time", &mut self.button_time)
                        .format(self.time_format)
                        .config(config.clone()),
                );
                ui.monospace(self.button_time.to_string());
                ui.end_row();

                ui.label("Timestamp");
                ui.add(
                    DateTimePickerButton::new("datetime", &mut self.button_datetime)
                        .format(self.time_format)
                        .config(config.clone()),
                );
                ui.monospace(self.button_datetime.to_string());
                ui.end_row();
            });
    }

    fn calendar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Calendar");
            let config = self.config();
            let mut cal = Calendar::new("inline", &mut self.calendar)
                .kinds(&self.kinds)
                .selection(self.calendar_value)
                .config(config)
                .show_clear(true);
            let badge: &dyn Fn(NaiveDate) -> usize = &fake_entries;
            if self.badges {
                cal = cal.day_badge(badge);
            }
            let r = cal.show(ui);
            if let Some(p) = r.picked {
                self.calendar_value = Some(p);
                self.calendar_log.push(format!(
                    "{p}  ({} – {}, {} days)",
                    p.first_day(),
                    p.last_day(),
                    p.len_days()
                ));
            }
            if r.cleared {
                self.calendar_value = None;
                self.calendar_log.push("cleared".into());
            }
            if let Some(a) = self.calendar.anchor {
                ui.weak(format!("Range from {a}: click the other end (Esc cancels)"));
            }
            for line in self.calendar_log.iter().rev().take(5) {
                ui.monospace(line);
            }
        });
    }

    fn drums(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Drums");
            ui.label("One drum, 0–10, no wrap:");
            ui.add(Drum::new("single", &mut self.drum_value, 0..=10));
            ui.monospace(format!("value = {}", self.drum_value));
            ui.add_space(8.0);
            ui.label("A time:");
            let config = self.config();
            ui.add(
                TimeDrum::new("time_drum", &mut self.time)
                    .seconds(self.time_format.seconds)
                    .hour12(self.time_format.hour12)
                    .minute_step(self.time_format.minute_step)
                    .labels(config.labels.clone()),
            );
            ui.monospace(self.time.to_string());
        });
    }

    fn datetime(&mut self, ui: &mut egui::Ui) {
        ui.heading("Calendar and drums");
        let config = self.config();
        ui.add(
            DateTimePicker::new("dt_inline", &mut self.datetime, &mut self.datetime_state)
                .format(self.time_format)
                .config(config),
        );
        ui.monospace(self.datetime.to_string());
    }
}
