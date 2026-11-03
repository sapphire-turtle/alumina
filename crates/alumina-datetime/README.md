# alumina-datetime

[egui](https://github.com/emilk/egui) widgets for picking dates, periods, times and
timestamps, on the desktops and on phones.

- **`Calendar`**: a month's days that picks a day, an ISO week, a month, a year or a range
  of days (the kinds you allow, switched above the grid). The heading zooms out to the
  months and the decade. Week numbers, a dot per day (entries on it, say), Today and Clear.
- **`Drum`** / **`TimeDrum`**: a rolling column — drag it (it rolls on after a flick), turn
  the wheel, click a row or press ↑ / ↓ — that settles on the middle value; a time of day on
  a few of them, in 24 hours or 12 with AM / PM, minutes in steps.
- **`DateTimePicker`**: the calendar beside the drums, for a timestamp (under it on a
  narrow screen).
- **`DatePickerButton`**, **`PeriodPickerButton`**, **`TimePickerButton`**,
  **`DateTimePickerButton`**: the value on a button that opens the above in a popup, with a
  text field to type it.
- **`Period`**: a span of days, read and written as `2026-11-03`, `2026-W45`, `2026-11`,
  `2026` or `2026-11-01..2026-11-10`.

```rust
use alumina_datetime::{Period, PeriodKind, PeriodPickerButton};

fn filter(ui: &mut egui::Ui, period: &mut Option<Period>) {
    if ui.add(PeriodPickerButton::new("filter", period).kinds(&PeriodKind::ALL)).changed() {
        // Show what falls in `period`.
    }
}
```

## Words and colours

Every word the widgets show comes from `Labels` (English by default), passed with the first
day of the week and the other shared settings in a `CalendarConfig`: hand it your app's
translation. The colours are egui's `Visuals` (`selection.bg_fill` for what is picked, and
so on), so your theme applies; change them for one widget with `ui.scope`.

## Timestamps

Timestamps are `chrono::NaiveDateTime`, wall time: convert to and from a time zone in your
app.

## Versions

| alumina-datetime | egui |
|---|---|
| 0.1 | 0.37 |

## Licence

MIT or Apache-2.0, at your option.
