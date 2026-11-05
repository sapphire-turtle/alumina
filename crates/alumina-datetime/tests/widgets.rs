//! The widgets driven as a user would: clicks, keys, popups.
//!
//! With `ALUMINA_RENDER_DIR` set, each test also renders its window there as a PNG (on the
//! GPU, or a software Vulkan such as lavapipe), for looking at.

use alumina_datetime::chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use alumina_datetime::{
    Calendar, CalendarConfig, CalendarIcons, CalendarState, CalendarView, DatePickerButton,
    DateTimePicker, DateTimePickerButton, Drum, Period, PeriodKind, PeriodPickerButton, TimeDrum,
    TimePickerButton,
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

#[test]
fn datetime_picker_stacks_on_a_narrow_screen() {
    let at = d(2026, 11, 3).and_hms_opt(9, 30, 0).unwrap();
    for (width, name) in [(600.0, "datetime_wide"), (300.0, "datetime_narrow")] {
        let mut h = Harness::builder().with_size([width, 520.0]).build_ui_state(
            |ui, (dt, state): &mut (NaiveDateTime, CalendarState)| {
                ui.add(DateTimePicker::new("dt", dt, state).config(config()));
            },
            (at, CalendarState::new(at.date())),
        );
        h.run();
        let day = h.get_by_label("2026-11-03").rect();
        let minute = h.get_by_label("30").rect();
        if width > 400.0 {
            assert!(minute.left() > day.right(), "the drums beside the calendar");
        } else {
            assert!(minute.top() > day.bottom(), "the drums under the calendar");
        }
        render(&mut h, name);
    }
}

const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path d="M3 2h6a3 3 0 0 1 0 6H3z" fill="none" stroke="currentColor"/></svg>"#;

type Seen = std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<Recorder>>>>;

fn icon(name: &str) -> Option<egui::ImageSource<'static>> {
    Some(egui::ImageSource::Bytes {
        uri: format!("bytes://{name}.svg").into(),
        bytes: SVG.as_bytes().into(),
    })
}

/// Every icon but `next`, whose chevron stays drawn.
fn all_icons() -> CalendarIcons {
    let mut icons = CalendarIcons::default();
    icons.previous = icon("previous");
    icons.today = icon("today");
    icons.clear = icon("clear");
    icons.now = icon("now");
    icons
}

/// An image loader that records the URIs it is asked for and loads nothing.
#[derive(Default)]
struct Recorder(std::sync::Mutex<Vec<String>>);

impl egui::load::ImageLoader for Recorder {
    fn id(&self) -> &str {
        "recorder"
    }
    fn load(
        &self,
        _: &egui::Context,
        uri: &str,
        _: egui::load::SizeHint,
    ) -> egui::load::ImageLoadResult {
        let mut seen = self.0.lock().unwrap();
        if !seen.iter().any(|u| u == uri) {
            seen.push(uri.to_owned());
        }
        Err(egui::load::LoadError::NotSupported)
    }
    fn forget(&self, _: &str) {}
    fn forget_all(&self) {}
    fn byte_size(&self) -> usize {
        0
    }
}

/// Installs the recorder (once) in front of the SVG loader.
fn record(seen: &Seen, ctx: &egui::Context) {
    seen.lock().unwrap().get_or_insert_with(|| {
        let r = std::sync::Arc::new(Recorder::default());
        // The most recently added loader is asked first.
        egui_extras::install_image_loaders(ctx);
        ctx.add_image_loader(r.clone());
        r
    });
}

fn requested(seen: &Seen) -> Vec<String> {
    let mut v = seen
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .0
        .lock()
        .unwrap()
        .clone();
    v.sort();
    v
}

fn uris(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| format!("bytes://{n}.svg")).collect()
}

fn with_icons(icons: CalendarIcons) -> (Harness<'static, Cal>, Seen) {
    let state = Cal {
        state: CalendarState::for_period(None, &[PeriodKind::Day], d(2026, 11, 3)),
        kinds: vec![PeriodKind::Day],
        value: None,
    };
    let seen = Seen::default();
    let seen2 = seen.clone();
    let h = Harness::builder().with_size([320.0, 330.0]).build_ui_state(
        move |ui, s: &mut Cal| {
            record(&seen2, ui.ctx());
            let r = Calendar::new("cal", &mut s.state)
                .kinds(&s.kinds)
                .selection(s.value)
                .config(config().icons(icons.clone()))
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
    );
    (h, seen)
}

#[test]
fn without_icons_no_image_is_requested_and_the_buttons_work() {
    let (mut h, seen) = with_icons(CalendarIcons::default());
    h.run();
    h.get_by_label("Previous").click();
    h.run();
    assert_eq!(h.state().state.cursor, d(2026, 10, 3));
    h.get_by_label("Today").click();
    h.run();
    assert_eq!(h.state().value, Some(Period::Day(d(2026, 11, 3))));
    h.get_by_label("Clear").click();
    h.run();
    assert_eq!(h.state().value, None);
    assert!(requested(&seen).is_empty());
}

#[test]
fn icon_buttons_load_their_images_and_act_as_the_text_ones() {
    let (mut h, seen) = with_icons(all_icons());
    h.run();
    // Previous, Today and Clear; not Now (not in a calendar) and not Next (no icon).
    assert_eq!(requested(&seen), uris(&["clear", "previous", "today"]));
    h.get_by_label("Previous").click();
    h.run();
    assert_eq!(h.state().state.cursor, d(2026, 10, 3));
    // The next arrow has no icon: still drawn, still named.
    h.get_by_label("Next").click();
    h.run();
    assert_eq!(h.state().state.cursor, d(2026, 11, 3));
    h.get_by_label("Today").click();
    h.run();
    assert_eq!(h.state().value, Some(Period::Day(d(2026, 11, 3))));
    h.get_by_label("Clear").click();
    h.run();
    assert_eq!(h.state().value, None);
    render(&mut h, "calendar_icons");
}

#[test]
fn icon_buttons_show_their_label_as_hover_text() {
    let (mut h, _) = with_icons(all_icons());
    h.run();
    h.get_by_label("Today").hover();
    for _ in 0..10 {
        h.step();
    }
    // The button, and its tooltip.
    assert_eq!(h.get_all_by_label("Today").count(), 2);
}

/// The Now button of a time picker's popup: what images were loaded, and how tall it is.
fn now_button(icons: CalendarIcons, datetime: bool) -> (Vec<String>, f32) {
    let seen = Seen::default();
    let seen2 = seen.clone();
    let mut h = Harness::builder()
        .with_size([420.0, 520.0])
        .build_ui(move |ui| {
            record(&seen2, ui.ctx());
            let config = config().icons(icons.clone());
            if datetime {
                let mut v = d(2026, 11, 3).and_hms_opt(9, 30, 0).unwrap();
                ui.add(DateTimePickerButton::new("dt", &mut v).config(config));
            } else {
                let mut v = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
                ui.add(TimePickerButton::new("t", &mut v).config(config));
            }
        });
    h.run();
    h.get_by_label(if datetime {
        "2026-11-03 09:30"
    } else {
        "09:30"
    })
    .click();
    h.run();
    h.run();
    let height = h.get_by_label("Now").rect().height();
    (requested(&seen), height)
}

#[test]
fn the_now_icon_loads_and_keeps_the_buttons_height() {
    for datetime in [false, true] {
        let (none, plain) = now_button(CalendarIcons::default(), datetime);
        assert!(none.is_empty());
        let (loaded, with_icon) = now_button(all_icons(), datetime);
        assert!(loaded.contains(&"bytes://now.svg".to_owned()), "{loaded:?}");
        assert!(
            (with_icon - plain).abs() < 0.5,
            "{with_icon} against {plain} (datetime: {datetime})"
        );
    }
}

#[test]
fn the_today_and_clear_icons_keep_the_buttons_height() {
    let (mut plain, _) = with_icons(CalendarIcons::default());
    plain.run();
    let (mut iconed, _) = with_icons(all_icons());
    iconed.run();
    for name in ["Today", "Clear"] {
        let a = plain.get_by_label(name).rect().height();
        let b = iconed.get_by_label(name).rect().height();
        assert!((a - b).abs() < 0.5, "{name}: {b} against {a}");
    }
}

#[test]
fn a_non_square_chevron_icon_leaves_the_button_alone() {
    let wide = r#"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="24" viewBox="0 0 48 24"><path d="M4 4h40v16H4z"/></svg>"#;
    let rect = |icons: CalendarIcons| {
        let (mut h, _) = with_icons(icons);
        h.run();
        h.run();
        h.get_by_label("Previous").rect()
    };
    let mut icons = CalendarIcons::default();
    icons.previous = Some(egui::ImageSource::Bytes {
        uri: "bytes://wide.svg".into(),
        bytes: wide.as_bytes().into(),
    });
    assert_eq!(rect(icons), rect(CalendarIcons::default()));
}
