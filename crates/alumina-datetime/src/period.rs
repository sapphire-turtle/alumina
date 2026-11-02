//! A span of days: one day, a week, a month, a year or any range of days.

use std::fmt;
use std::str::FromStr;

use chrono::{Datelike as _, Days, NaiveDate, Weekday};

use crate::calc::{days_in_month, week_start};

/// What a [`Period`] spans; also what a calendar picks (see [`Calendar`](crate::Calendar)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PeriodKind {
    Day,
    Week,
    Month,
    Year,
    /// Any days from one to another, picked with two clicks.
    Range,
}

impl PeriodKind {
    /// Every kind, finest first.
    pub const ALL: [PeriodKind; 5] = [
        PeriodKind::Day,
        PeriodKind::Week,
        PeriodKind::Month,
        PeriodKind::Year,
        PeriodKind::Range,
    ];
}

/// A span of whole days.
///
/// Written and read (`Display`, `FromStr`) as:
///
/// | Period | Text |
/// |---|---|
/// | a day | `2026-11-03` |
/// | an ISO week (from Monday) | `2026-W45` |
/// | a month | `2026-11` |
/// | a year | `2026` |
/// | a range, both ends included | `2026-11-01..2026-11-10` (also `2026-11-01/2026-11-10`) |
///
/// A week starting on another day than Monday has no ISO name and is written as its range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Period {
    Day(NaiveDate),
    /// The seven days from `start`, the first day of the week.
    Week {
        start: NaiveDate,
    },
    Month {
        year: i32,
        month: u32,
    },
    Year(i32),
    /// From `start` to `end`, both included; `start <= end` (see [`Period::range`]).
    Range {
        start: NaiveDate,
        end: NaiveDate,
    },
}

impl Period {
    /// The week `date` falls in, weeks starting on `first`.
    pub fn week_of(date: NaiveDate, first: Weekday) -> Self {
        Period::Week {
            start: week_start(date, first),
        }
    }

    /// The ISO week `week` of ISO year `year`, from its Monday.
    pub fn iso_week(year: i32, week: u32) -> Option<Self> {
        NaiveDate::from_isoywd_opt(year, week, Weekday::Mon).map(|start| Period::Week { start })
    }

    /// The month `date` falls in.
    pub fn month_of(date: NaiveDate) -> Self {
        Period::Month {
            year: date.year(),
            month: date.month(),
        }
    }

    /// The days from `a` to `b` in whichever order they come.
    pub fn range(a: NaiveDate, b: NaiveDate) -> Self {
        Period::Range {
            start: a.min(b),
            end: a.max(b),
        }
    }

    /// The period of `kind` that contains `date` (for a range, `date` alone); weeks start on
    /// `first`.
    pub fn containing(kind: PeriodKind, date: NaiveDate, first: Weekday) -> Self {
        match kind {
            PeriodKind::Day => Period::Day(date),
            PeriodKind::Week => Period::week_of(date, first),
            PeriodKind::Month => Period::month_of(date),
            PeriodKind::Year => Period::Year(date.year()),
            PeriodKind::Range => Period::range(date, date),
        }
    }

    pub fn kind(&self) -> PeriodKind {
        match self {
            Period::Day(_) => PeriodKind::Day,
            Period::Week { .. } => PeriodKind::Week,
            Period::Month { .. } => PeriodKind::Month,
            Period::Year(_) => PeriodKind::Year,
            Period::Range { .. } => PeriodKind::Range,
        }
    }

    /// The first day.
    pub fn first_day(&self) -> NaiveDate {
        match *self {
            Period::Day(d) => d,
            Period::Week { start } | Period::Range { start, .. } => start,
            Period::Month { year, month } => {
                NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(NaiveDate::MIN)
            }
            Period::Year(year) => NaiveDate::from_ymd_opt(year, 1, 1).unwrap_or(NaiveDate::MIN),
        }
    }

    /// The last day, included.
    pub fn last_day(&self) -> NaiveDate {
        match *self {
            Period::Day(d) => d,
            Period::Week { start } => start
                .checked_add_days(Days::new(6))
                .unwrap_or(NaiveDate::MAX),
            Period::Range { end, .. } => end,
            Period::Month { year, month } => {
                NaiveDate::from_ymd_opt(year, month, days_in_month(year, month))
                    .unwrap_or(NaiveDate::MAX)
            }
            Period::Year(year) => NaiveDate::from_ymd_opt(year, 12, 31).unwrap_or(NaiveDate::MAX),
        }
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        self.first_day() <= date && date <= self.last_day()
    }

    /// The number of days.
    pub fn len_days(&self) -> u64 {
        (self.last_day() - self.first_day()).num_days() as u64 + 1
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Period::Day(d) => write!(f, "{}", d.format("%Y-%m-%d")),
            Period::Week { start } if start.weekday() == Weekday::Mon => {
                let w = start.iso_week();
                write!(f, "{}-W{:02}", w.year(), w.week())
            }
            Period::Week { .. } | Period::Range { .. } => write!(
                f,
                "{}..{}",
                self.first_day().format("%Y-%m-%d"),
                self.last_day().format("%Y-%m-%d")
            ),
            Period::Month { year, month } => write!(f, "{year:04}-{month:02}"),
            Period::Year(year) => write!(f, "{year:04}"),
        }
    }
}

/// Why a text is not a [`Period`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsePeriodError(String);

impl fmt::Display for ParsePeriodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a date or period: {:?}", self.0)
    }
}

impl std::error::Error for ParsePeriodError {}

impl FromStr for Period {
    type Err = ParsePeriodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let err = || ParsePeriodError(s.to_string());
        if let Some((a, b)) = s.split_once("..").or_else(|| s.split_once('/')) {
            let a = parse_day(a.trim()).ok_or_else(err)?;
            let b = parse_day(b.trim()).ok_or_else(err)?;
            return Ok(Period::range(a, b));
        }
        if let Some(d) = parse_day(s) {
            return Ok(Period::Day(d));
        }
        if let Some((y, w)) = s.split_once(['W', 'w']) {
            let year = y
                .strip_suffix('-')
                .unwrap_or(y)
                .parse()
                .map_err(|_| err())?;
            let week = w.parse().map_err(|_| err())?;
            return Period::iso_week(year, week).ok_or_else(err);
        }
        if let Some((y, m)) = s.split_once('-') {
            let year: i32 = y.parse().map_err(|_| err())?;
            let month: u32 = m.parse().map_err(|_| err())?;
            if (1..=12).contains(&month) && y.len() == 4 {
                return Ok(Period::Month { year, month });
            }
            return Err(err());
        }
        if s.len() == 4
            && let Ok(year) = s.parse()
        {
            return Ok(Period::Year(year));
        }
        Err(err())
    }
}

fn parse_day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn round_trips() {
        for text in [
            "2026-11-03",
            "2026-W45",
            "2026-W01",
            "2026-11",
            "2026",
            "2026-11-01..2026-11-10",
        ] {
            let p: Period = text.parse().unwrap();
            assert_eq!(p.to_string(), text, "{p:?}");
        }
    }

    #[test]
    fn parses_variants() {
        assert_eq!(
            "2026-11-10/2026-11-01".parse::<Period>().unwrap(),
            Period::range(d(2026, 11, 1), d(2026, 11, 10))
        );
        assert_eq!(
            "2026w45".parse::<Period>().unwrap(),
            Period::Week {
                start: d(2026, 11, 2)
            }
        );
        for bad in [
            "",
            "2026-13",
            "2026-W54",
            "26",
            "today",
            "2026-11-31",
            "1..2",
        ] {
            assert!(bad.parse::<Period>().is_err(), "{bad}");
        }
    }

    #[test]
    fn bounds() {
        let p = Period::Month {
            year: 2024,
            month: 2,
        };
        assert_eq!(p.first_day(), d(2024, 2, 1));
        assert_eq!(p.last_day(), d(2024, 2, 29));
        assert_eq!(p.len_days(), 29);
        assert!(p.contains(d(2024, 2, 29)));
        assert!(!p.contains(d(2024, 3, 1)));
        assert_eq!(Period::Year(2026).len_days(), 365);
        // The ISO year of 2027-01-01 (a Friday) is 2026.
        assert_eq!(
            Period::week_of(d(2027, 1, 1), Weekday::Mon).to_string(),
            "2026-W53"
        );
    }

    #[test]
    fn sunday_weeks_are_written_as_ranges() {
        let p = Period::week_of(d(2026, 11, 3), Weekday::Sun);
        assert_eq!(p.to_string(), "2026-11-01..2026-11-07");
        assert_eq!(p.len_days(), 7);
    }

    #[test]
    fn containing() {
        let day = d(2026, 11, 3);
        for kind in PeriodKind::ALL {
            let p = Period::containing(kind, day, Weekday::Mon);
            assert_eq!(p.kind(), kind);
            assert!(p.contains(day));
        }
    }
}
