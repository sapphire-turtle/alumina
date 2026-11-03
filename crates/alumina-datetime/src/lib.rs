//! egui widgets for picking dates, periods, times and timestamps.
//!
//! - [`Calendar`]: a month's days that picks a day, a week, a month, a year or a range of
//!   days, zooming out to the months and the years.
//! - [`Drum`] and [`TimeDrum`]: values on a rolling column, a time of day on a few of them.
//! - [`DateTimePicker`]: a calendar beside time drums, for a timestamp.
//! - [`DatePickerButton`], [`PeriodPickerButton`], [`TimePickerButton`],
//!   [`DateTimePickerButton`]: a button reading the value that opens the above in a popup,
//!   with a text field to type it.
//! - [`Period`]: a span of days, written `2026-11-03`, `2026-W45`, `2026-11`, `2026` or
//!   `2026-11-01..2026-11-10`.
//!
//! Every word shown comes from [`Labels`] (English by default), passed in a
//! [`CalendarConfig`] with the first day of the week and the other shared settings.
//!
//! Timestamps are [`chrono::NaiveDateTime`], local wall time; the app converts to and from a
//! time zone.

pub mod calc;
mod calendar;
mod drum;
mod labels;
mod period;
mod picker;

pub use calendar::{Calendar, CalendarResponse, CalendarState, CalendarView, DayBadge};
pub use drum::{Drum, TimeDrum};
pub use labels::{CalendarConfig, Labels};
pub use period::{ParsePeriodError, Period, PeriodKind};
pub use picker::{
    DatePickerButton, DateTimePicker, DateTimePickerButton, PeriodPickerButton, TimeFormat,
    TimePickerButton,
};

pub use chrono;
pub use egui;
