//! Calendar arithmetic the widgets and [`Period`](crate::Period) share.

use jiff::Span;
use jiff::civil::{Date, Weekday};

/// The day `day` of `month` of `year`, when there is one (and `year` is within jiff's
/// -9999–9999).
pub(crate) fn ymd(year: i32, month: u32, day: u32) -> Option<Date> {
    Date::new(
        i16::try_from(year).ok()?,
        i8::try_from(month).ok()?,
        i8::try_from(day).ok()?,
    )
    .ok()
}

/// The number of days in `month` (1–12) of `year`.
///
/// # Panics
/// When `month` is not 1–12.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    assert!((1..=12).contains(&month), "month out of range: {month}");
    ymd(year, month, 1).map_or(31, |d| d.days_in_month() as u32)
}

/// `date` moved by `months` calendar months, its day clamped to the last of the target month
/// (Jan 31 + 1 month is Feb 28 or 29).
pub fn add_months(date: Date, months: i32) -> Date {
    Span::new()
        .try_months(months)
        .ok()
        .and_then(|span| date.checked_add(span).ok())
        .unwrap_or(date)
}

/// `date` moved by `years` years, Feb 29 becoming Feb 28 in a common year.
pub fn add_years(date: Date, years: i32) -> Date {
    add_months(date, years.saturating_mul(12))
}

/// `date` moved by `days` days, stopping at the first or last day jiff has.
pub(crate) fn add_days(date: Date, days: i64) -> Date {
    Span::new()
        .try_days(days)
        .map_or(date, |span| date.saturating_add(span))
}

/// The first day of the week `date` falls in, weeks starting on `first`.
pub fn week_start(date: Date, first: Weekday) -> Date {
    let back = date.weekday().since(first);
    add_days(date, -i64::from(back))
}

/// The first day of `date`'s month.
pub fn month_start(date: Date) -> Date {
    date.first_of_month()
}

/// The weekdays of a week starting on `first`, in order.
pub fn weekdays_from(first: Weekday) -> [Weekday; 7] {
    let mut out = [first; 7];
    for i in 1..7 {
        out[i] = out[i - 1].next();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i16, m: i8, day: i8) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn month_lengths() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(days_in_month(2026, 4), 30);
    }

    #[test]
    fn months_clamp_the_day() {
        assert_eq!(add_months(d(2026, 1, 31), 1), d(2026, 2, 28));
        assert_eq!(add_months(d(2024, 1, 31), 1), d(2024, 2, 29));
        assert_eq!(add_months(d(2026, 1, 15), -1), d(2025, 12, 15));
        assert_eq!(add_months(d(2026, 11, 3), 14), d(2028, 1, 3));
        assert_eq!(add_years(d(2024, 2, 29), 1), d(2025, 2, 28));
    }

    #[test]
    fn week_starts() {
        // 2026-11-03 is a Tuesday.
        assert_eq!(week_start(d(2026, 11, 3), Weekday::Monday), d(2026, 11, 2));
        assert_eq!(week_start(d(2026, 11, 3), Weekday::Sunday), d(2026, 11, 1));
        assert_eq!(week_start(d(2026, 11, 1), Weekday::Monday), d(2026, 10, 26));
        assert_eq!(week_start(d(2026, 11, 2), Weekday::Monday), d(2026, 11, 2));
        assert_eq!(weekdays_from(Weekday::Sunday)[6], Weekday::Saturday);
    }
}
