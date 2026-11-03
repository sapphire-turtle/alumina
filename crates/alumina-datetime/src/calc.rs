//! Calendar arithmetic the widgets and [`Period`](crate::Period) share.

use chrono::{Datelike as _, Days, NaiveDate, Weekday};

/// The number of days in `month` (1–12) of `year`.
///
/// # Panics
/// When `month` is not 1–12.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    assert!((1..=12).contains(&month), "month out of range: {month}");
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map_or(31, |d| d.day())
}

/// `date` moved by `months` calendar months, its day clamped to the last of the target month
/// (Jan 31 + 1 month is Feb 28 or 29).
pub fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let total = date.year() * 12 + date.month0() as i32 + months;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) as u32 + 1;
    let day = date.day().min(days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(date)
}

/// `date` moved by `years` years, Feb 29 becoming Feb 28 in a common year.
pub fn add_years(date: NaiveDate, years: i32) -> NaiveDate {
    add_months(date, years * 12)
}

/// The first day of the week `date` falls in, weeks starting on `first`.
pub fn week_start(date: NaiveDate, first: Weekday) -> NaiveDate {
    let back = (7 + date.weekday().num_days_from_monday() - first.num_days_from_monday()) % 7;
    date - Days::new(u64::from(back))
}

/// The first day of `date`'s month.
pub fn month_start(date: NaiveDate) -> NaiveDate {
    date.with_day(1).unwrap_or(date)
}

/// The weekdays of a week starting on `first`, in order.
pub fn weekdays_from(first: Weekday) -> [Weekday; 7] {
    let mut out = [first; 7];
    for i in 1..7 {
        out[i] = out[i - 1].succ();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
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
        assert_eq!(week_start(d(2026, 11, 3), Weekday::Mon), d(2026, 11, 2));
        assert_eq!(week_start(d(2026, 11, 3), Weekday::Sun), d(2026, 11, 1));
        assert_eq!(week_start(d(2026, 11, 1), Weekday::Mon), d(2026, 10, 26));
        assert_eq!(week_start(d(2026, 11, 2), Weekday::Mon), d(2026, 11, 2));
        assert_eq!(weekdays_from(Weekday::Sun)[6], Weekday::Sat);
    }
}
