//! A span of days: one day, a week, a month, a year or any range of days.

use std::fmt;
use std::str::FromStr;

use jiff::civil::{Date, ISOWeekDate, Weekday};

use crate::calc::{add_days, days_in_month, week_start, ymd};

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
    Day(Date),
    /// The seven days from `start`, the first day of the week.
    Week {
        start: Date,
    },
    Month {
        year: i32,
        month: u32,
    },
    Year(i32),
    /// From `start` to `end`, both included; `start <= end` (see [`Period::range`]).
    Range {
        start: Date,
        end: Date,
    },
}

impl Period {
    /// The week `date` falls in, weeks starting on `first`.
    pub fn week_of(date: Date, first: Weekday) -> Self {
        Period::Week {
            start: week_start(date, first),
        }
    }

    /// The ISO week `week` of ISO year `year`, from its Monday.
    pub fn iso_week(year: i32, week: u32) -> Option<Self> {
        let year = i16::try_from(year).ok()?;
        let week = i8::try_from(week).ok()?;
        let start = ISOWeekDate::new(year, week, Weekday::Monday).ok()?.date();
        Some(Period::Week { start })
    }

    /// The month `date` falls in.
    pub fn month_of(date: Date) -> Self {
        Period::Month {
            year: i32::from(date.year()),
            month: date.month() as u32,
        }
    }

    /// The days from `a` to `b` in whichever order they come.
    pub fn range(a: Date, b: Date) -> Self {
        Period::Range {
            start: a.min(b),
            end: a.max(b),
        }
    }

    /// The period of `kind` that contains `date` (for a range, `date` alone); weeks start on
    /// `first`.
    pub fn containing(kind: PeriodKind, date: Date, first: Weekday) -> Self {
        match kind {
            PeriodKind::Day => Period::Day(date),
            PeriodKind::Week => Period::week_of(date, first),
            PeriodKind::Month => Period::month_of(date),
            PeriodKind::Year => Period::Year(i32::from(date.year())),
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
    pub fn first_day(&self) -> Date {
        match *self {
            Period::Day(d) => d,
            Period::Week { start } | Period::Range { start, .. } => start,
            Period::Month { year, month } => ymd(year, month, 1).unwrap_or(Date::MIN),
            Period::Year(year) => ymd(year, 1, 1).unwrap_or(Date::MIN),
        }
    }

    /// The last day, included.
    pub fn last_day(&self) -> Date {
        match *self {
            Period::Day(d) => d,
            Period::Week { start } => add_days(start, 6),
            Period::Range { end, .. } => end,
            Period::Month { year, month } => {
                ymd(year, month, days_in_month(year, month)).unwrap_or(Date::MAX)
            }
            Period::Year(year) => ymd(year, 12, 31).unwrap_or(Date::MAX),
        }
    }

    pub fn contains(&self, date: Date) -> bool {
        self.first_day() <= date && date <= self.last_day()
    }

    /// The number of days.
    pub fn len_days(&self) -> u64 {
        self.first_day()
            .until(self.last_day())
            .map_or(0, |span| span.get_days() as u64)
            + 1
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Period::Day(d) => write!(f, "{}", d.strftime(DAY_FMT)),
            Period::Week { start } if start.weekday() == Weekday::Monday => {
                let w = start.iso_week_date();
                write!(f, "{}-W{:02}", w.year(), w.week())
            }
            Period::Week { .. } | Period::Range { .. } => write!(
                f,
                "{}..{}",
                self.first_day().strftime(DAY_FMT),
                self.last_day().strftime(DAY_FMT)
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

const DAY_FMT: &str = "%Y-%m-%d";

fn parse_day(s: &str) -> Option<Date> {
    Date::strptime(DAY_FMT, s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i16, m: i8, day: i8) -> Date {
        Date::new(y, m, day).unwrap()
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
            Period::week_of(d(2027, 1, 1), Weekday::Monday).to_string(),
            "2026-W53"
        );
    }

    #[test]
    fn sunday_weeks_are_written_as_ranges() {
        let p = Period::week_of(d(2026, 11, 3), Weekday::Sunday);
        assert_eq!(p.to_string(), "2026-11-01..2026-11-07");
        assert_eq!(p.len_days(), 7);
    }

    #[test]
    fn containing() {
        let day = d(2026, 11, 3);
        for kind in PeriodKind::ALL {
            let p = Period::containing(kind, day, Weekday::Monday);
            assert_eq!(p.kind(), kind);
            assert!(p.contains(day));
        }
    }
}
