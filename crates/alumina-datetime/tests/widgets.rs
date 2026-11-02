//! The widgets driven as a user would: clicks, keys, popups.
//!
//! With `ALUMINA_RENDER_DIR` set, each test also renders its window there as a PNG (on the
//! GPU, or a software Vulkan such as lavapipe), for looking at.

use alumina_datetime::chrono::{NaiveDate, NaiveTime};
use alumina_datetime::{
    Calendar, CalendarConfig, CalendarState, CalendarView, DatePickerButton, Drum, Period,
    PeriodKind, PeriodPickerButton, TimeDrum,
};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn config() -> CalendarConfig {
    CalendarConfig::default()
        .today(d(2026, 11, 3))
        .show_week_numbers(true)
}

fn render(harness: &mut Harness<'_, impl Sized>, name: &str) {
    let Ok(dir) = std::env::var("ALUMINA_RENDER_DIR") else {
        return;
    };
    harness.run();
    let image = harness.render().expect("render");
    image.save(format!("{dir}/{name}.png")).expect("save");
}

struct Cal {
    state: CalendarState,
    kinds: Vec<PeriodKind>,
    value: Option<Period>,
}

fn calendar(kinds: &[PeriodKind]) -> Harness<'static, Cal> {
    let state = Cal {
        state: CalendarState::for_period(None, kinds, d(2026, 11, 3)),
        kinds: kinds.to_vec(),
        value: None,
    };
    Harness::builder().with_size([320.0, 330.0]).build_ui_state(
        |ui, s: &mut Cal| {
            let r = Calendar::new("cal", &mut s.state)
                .kinds(&s.kinds)
                .selection(s.value)
                .config(config())
                .show_clear(true)
                .show(ui);
            if let Some(p) = r.picked {
                s.value = Some(p);
            }
            if r.cleared {
                s.value = None;
            }
        },
        state,
    )
}

#[test]
fn picks_a_day() {
    let mut h = calendar(&[PeriodKind::Day]);
    h.get_by_label("2026-11-15").click();
    h.run();
    assert_eq!(h.state().value, Some(Period::Day(d(2026, 11, 15))));
    render(&mut h, "calendar_day");
}

#[test]
fn picks_a_week_from_any_of_its_days() {
    let mut h = calendar(&PeriodKind::ALL);
    h.get_by_label("Week").click();
    h.run();
    h.get_by_label("2026-11-12").click();
    h.run();
    assert_eq!(h.state().value, Some("2026-W46".parse().unwrap()));
    // And from the week-number column.
    h.get_by_label("2026-W45").click();
    h.run();
    assert_eq!(h.state().value, Some("2026-W45".parse().unwrap()));
    render(&mut h, "calendar_week");
}

#[test]
fn picks_a_range_with_two_clicks_in_either_order() {
    let mut h = calendar(&[PeriodKind::Range]);
    h.get_by_label("2026-11-20").click();
    h.run();
    assert_eq!(h.state().state.anchor, Some(d(2026, 11, 20)));
    assert_eq!(h.state().value, None);
    h.get_by_label("2026-11-09").click();
    h.run();
    assert_eq!(
        h.state().value,
        Some(Period::range(d(2026, 11, 9), d(2026, 11, 20)))
    );
    render(&mut h, "calendar_range");
}

#[test]
fn picks_a_month_and_a_year() {
    let mut h = calendar(&PeriodKind::ALL);
    h.get_by_label("Month").click();
    h.run();
    assert_eq!(h.state().state.view, CalendarView::Months);
    h.get_by_label("2026-02").click();
    h.run();
    assert_eq!(h.state().value, Some("2026-02".parse().unwrap()));
    render(&mut h, "calendar_month");

    h.get_by_label("Year").click();
    h.run();
    h.get_by_label("2029").click();
    h.run();
    assert_eq!(h.state().value, Some(Period::Year(2029)));
    render(&mut h, "calendar_year");
}

#[test]
fn zooms_out_and_back_in_to_a_far_day() {
    let mut h = calendar(&[PeriodKind::Day]);
    h.get_by_label("November 2026").click();
    h.run();
    assert_eq!(h.state().state.view, CalendarView::Months);
    h.get_by_label("2026").click();
    h.run();
    assert_eq!(h.state().state.view, CalendarView::Years);
    h.get_by_label("2019").click(); // Shown before the 2020s.
    h.run();
    h.get_by_label("2019-03").click();
    h.run();
    assert_eq!(h.state().state.view, CalendarView::Days);
    h.get_by_label("2019-03-14").click();
    h.run();
    assert_eq!(h.state().value, Some(Period::Day(d(2019, 3, 14))));
}

#[test]
fn today_and_clear() {
    let mut h = calendar(&PeriodKind::ALL);
    h.get_by_label("Month").click();
    h.run();
    h.get_by_label("Today").click();
    h.run();
    assert_eq!(h.state().value, Some("2026-11".parse().unwrap()));
    h.get_by_label("Clear").click();
    h.run();
    assert_eq!(h.state().value, None);
}

#[test]
fn drum_rolls_with_keys_and_clicks() {
    let mut h = Harness::builder().with_size([120.0, 200.0]).build_ui_state(
        |ui, v: &mut i32| {
            ui.add(
                Drum::new("drum", v, 0..=59)
                    .step(5)
                    .wrap(true)
                    .format(|v| format!("{v:02}")),
            );
        },
        10,
    );
    h.run();
    // A click on the drum focuses it (and, in the middle, rolls nowhere).
    h.get_by_label("10").click();
    h.run();
    assert_eq!(*h.state(), 10);
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(*h.state(), 15);
    h.key_press(egui::Key::ArrowUp);
    h.key_press(egui::Key::ArrowUp);
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(*h.state(), 0);
    // Wraps from 0 to 55.
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(*h.state(), 55);
    render(&mut h, "drum");
}

#[test]
fn time_drum_follows_the_value() {
    let mut h = Harness::builder().with_size([260.0, 200.0]).build_ui_state(
        |ui, t: &mut NaiveTime| {
            ui.add(TimeDrum::new("t", t).hour12(true));
        },
        NaiveTime::from_hms_opt(21, 35, 0).unwrap(),
    );
    h.run();
    h.get_by_label("09");
    h.get_by_label("35");
    h.get_by_label("PM");
    render(&mut h, "time_drum");
}

#[test]
fn date_button_opens_a_calendar_and_closes_on_a_pick() {
    let mut h = Harness::builder().with_size([340.0, 420.0]).build_ui_state(
        |ui, date: &mut Option<NaiveDate>| {
            ui.add(DatePickerButton::new("date", date).config(config()));
        },
        Some(d(2026, 11, 3)),
    );
    h.get_by_label("2026-11-03").click();
    h.run();
    render(&mut h, "date_popup");
    h.get_by_label("2026-11-21").click();
    h.run();
    assert_eq!(*h.state(), Some(d(2026, 11, 21)));
    assert!(h.query_by_label("2026-11-22").is_none(), "the popup closed");
}

#[test]
fn period_button_takes_typed_text() {
    let mut h = Harness::builder().with_size([340.0, 460.0]).build_ui_state(
        |ui, p: &mut Option<Period>| {
            ui.add(PeriodPickerButton::new("period", p).config(config()));
        },
        None,
    );
    h.get_by_label("—").click();
    h.run();
    h.get_by_role(egui::accesskit::Role::TextInput).focus();
    h.run();
    h.get_by_role(egui::accesskit::Role::TextInput)
        .type_text("2026-W40");
    h.run();
    assert_eq!(*h.state(), Some("2026-W40".parse().unwrap()));
    render(&mut h, "period_popup");
}
