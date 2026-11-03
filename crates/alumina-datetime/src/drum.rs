//! A drum: a column of values that rolls under a finger, the wheel or the keys, and stops with
//! one value in the middle.

use std::ops::RangeInclusive;
use std::sync::Arc;

use chrono::{NaiveTime, Timelike as _};
use egui::{
    Align2, FontId, IdSalt, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Widget,
    WidgetInfo, vec2,
};

use crate::Labels;

/// A column of values that rolls: drag it (it keeps going for a while after a flick), turn
/// the wheel over it, click a value above or below the middle, or focus it and press ↑ / ↓.
/// It stops with one value in the middle band: the value.
///
/// ```no_run
/// # fn ui(ui: &mut egui::Ui, minute: &mut i32) {
/// ui.add(alumina_datetime::Drum::new("minute", minute, 0..=59).step(5).wrap(true));
/// # }
/// ```
pub struct Drum<'a> {
    id_salt: IdSalt,
    value: &'a mut i32,
    values: Vec<i32>,
    wrap: bool,
    format: Box<dyn Fn(i32) -> String + 'a>,
    width: f32,
    row_height: Option<f32>,
    visible_rows: usize,
}

/// How the drum is moving, kept in egui's memory between frames.
#[derive(Clone, Copy, Debug, Default)]
struct Motion {
    /// The row in the middle, fractional while rolling (an index into the values; may run
    /// past them when wrapping, taken modulo their number).
    pos: f32,
    /// Rows a second, after a flick.
    velocity: f32,
    /// The row it is settling on.
    target: Option<f32>,
    /// When the wheel last turned, to settle only once it stops.
    last_wheel: f64,
    /// The value the position was last read from, to follow changes made elsewhere.
    synced: i32,
}

impl<'a> Drum<'a> {
    /// The values of `range`, one apart.
    pub fn new(
        id_salt: impl egui::AsIdSalt,
        value: &'a mut i32,
        range: RangeInclusive<i32>,
    ) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            value,
            values: range.collect(),
            wrap: false,
            format: Box::new(|v| v.to_string()),
            width: 48.0,
            row_height: None,
            visible_rows: 5,
        }
    }

    /// Only every `step`-th value of the range from its start (minutes in fives, say).
    pub fn step(mut self, step: usize) -> Self {
        let step = step.max(1);
        self.values = self.values.into_iter().step_by(step).collect();
        self
    }

    /// The values to roll through, in order, in place of a range.
    pub fn values(mut self, values: Vec<i32>) -> Self {
        self.values = values;
        self
    }

    /// Whether the last value is followed by the first again (59 by 0), as on a clock.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// How a value is written.
    pub fn format(mut self, format: impl Fn(i32) -> String + 'a) -> Self {
        self.format = Box::new(format);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = Some(height);
        self
    }

    /// How many rows show, the value's in the middle (made odd).
    pub fn visible_rows(mut self, rows: usize) -> Self {
        self.visible_rows = rows.max(1) | 1;
        self
    }
}

/// The index of the value nearest `value` (the first of two as near).
pub(crate) fn nearest_index(values: &[i32], value: i32) -> usize {
    values
        .iter()
        .enumerate()
        .min_by_key(|(_, v)| (i64::from(**v) - i64::from(value)).abs())
        .map_or(0, |(i, _)| i)
}

/// `pos` rounded to a row and brought into `0..n` (wrapping) or clamped to it.
pub(crate) fn row_index(pos: f32, n: usize, wrap: bool) -> usize {
    let i = pos.round() as i64;
    if wrap {
        i.rem_euclid(n as i64) as usize
    } else {
        i.clamp(0, n as i64 - 1) as usize
    }
}

impl Widget for Drum<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Drum {
            id_salt,
            value,
            values,
            wrap,
            format,
            width,
            row_height,
            visible_rows,
        } = self;
        let row_h =
            row_height.unwrap_or_else(|| ui.text_style_height(&egui::TextStyle::Body) + 10.0);
        let size = vec2(width, row_h * visible_rows as f32);
        let id = ui.make_persistent_id(id_salt);
        let (_, rect) = ui.allocate_space(size);
        let mut response = ui.interact(rect, id, Sense::click_and_drag());
        let n = values.len();
        if n == 0 {
            return response;
        }
        let last = (n - 1) as f32;

        let now = ui.input(|i| i.time);
        let dt = ui.input(|i| i.stable_dt).clamp(0.001, 0.1);
        let mut m = ui.data(|d| d.get_temp::<Motion>(id)).unwrap_or(Motion {
            pos: nearest_index(&values, *value) as f32,
            synced: *value,
            ..Default::default()
        });
        let idle = !response.dragged() && m.velocity == 0.0 && m.target.is_none();
        if idle && m.synced != *value {
            m.pos = nearest_index(&values, *value) as f32;
        }
        let mut touched = false;

        // Dragging, then a flick.
        if response.drag_started() {
            m.velocity = 0.0;
            m.target = None;
            response.request_focus();
        }
        if response.dragged() {
            m.pos -= response.drag_delta().y / row_h;
            touched = true;
        }
        if response.drag_stopped() {
            let v = -ui.input(|i| i.pointer.velocity().y) / row_h;
            m.velocity = if v.abs() > 2.0 {
                v.clamp(-40.0, 40.0)
            } else {
                0.0
            };
            touched = true;
        }
        // The wheel, which this column takes from any scroll area around it.
        if response.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            if dy != 0.0 {
                ui.input_mut(|i| i.smooth_scroll_delta.y = 0.0);
                m.pos -= dy / row_h;
                m.velocity = 0.0;
                m.target = None;
                m.last_wheel = now;
                touched = true;
            }
        }
        // A click on a row away from the middle rolls it there.
        if response.clicked() {
            response.request_focus();
            if let Some(p) = response.interact_pointer_pos() {
                let rows = ((p.y - rect.center().y) / row_h).round();
                m.target = Some(m.target.unwrap_or(m.pos).round() + rows);
                m.velocity = 0.0;
                touched = true;
            }
        }
        if response.has_focus() {
            let (up, down) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                )
            });
            let step = i32::from(down) - i32::from(up);
            if step != 0 {
                m.target = Some(m.target.unwrap_or(m.pos).round() + step as f32);
                m.velocity = 0.0;
                touched = true;
            }
        }

        // Rolling on, then settling on the nearest row.
        if !response.dragged() {
            if m.velocity != 0.0 {
                m.pos += m.velocity * dt;
                m.velocity *= (-dt * 3.5).exp();
                if m.velocity.abs() < 3.0 {
                    m.velocity = 0.0;
                }
                touched = true;
            } else if now - m.last_wheel > 0.15 {
                let t = m.target.unwrap_or(m.pos.round());
                let diff = t - m.pos;
                if diff.abs() < 0.002 {
                    m.pos = t;
                    m.target = None;
                } else {
                    m.pos += diff * (1.0 - (-dt * 18.0).exp());
                    m.target = Some(t);
                    touched = true;
                }
            }
        }

        // The ends: around, or a stop.
        if wrap {
            let shift = (m.pos / n as f32).floor() * n as f32;
            m.pos -= shift;
            if let Some(t) = &mut m.target {
                *t -= shift;
            }
        } else {
            if m.pos < 0.0 || m.pos > last {
                m.velocity = 0.0;
            }
            m.pos = m.pos.clamp(0.0, last);
            if let Some(t) = &mut m.target {
                *t = t.clamp(0.0, last);
            }
        }

        if touched {
            let v = values[row_index(m.pos, n, wrap)];
            if v != *value {
                *value = v;
                response.mark_changed();
            }
        }
        m.synced = *value;
        let moving = response.dragged()
            || m.velocity != 0.0
            || m.target.is_some()
            || now - m.last_wheel <= 0.15;
        ui.data_mut(|d| d.insert_temp(id, m));
        if moving {
            ui.ctx().request_repaint();
        }

        let shown = format(*value);
        response.widget_info(|| WidgetInfo::labeled(Role::SpinButton, true, &shown));

        // Drawing.
        let visuals = ui.visuals();
        let painter = ui.painter_at(rect);
        let band = Rect::from_center_size(rect.center(), vec2(rect.width(), row_h));
        painter.rect_filled(band, 4.0, visuals.selection.bg_fill.gamma_multiply(0.3));
        if response.has_focus() {
            painter.rect_stroke(
                band,
                4.0,
                Stroke::new(1.0, visuals.selection.bg_fill),
                StrokeKind::Inside,
            );
        }
        let base = m.pos.round() as i64;
        let half = (visible_rows / 2 + 1) as i64;
        let font_size = ui.text_style_height(&egui::TextStyle::Body) + 2.0;
        for k in -half..=half {
            let i = base + k;
            let idx = if wrap {
                i.rem_euclid(n as i64) as usize
            } else if (0..n as i64).contains(&i) {
                i as usize
            } else {
                continue;
            };
            let offset = i as f32 - m.pos;
            let y = rect.center().y + offset * row_h;
            let dist = offset.abs();
            let fade = (1.0 - dist / (half as f32 + 0.5)).clamp(0.1, 1.0);
            let color = if dist < 0.5 {
                visuals.strong_text_color()
            } else {
                visuals.text_color().gamma_multiply(fade)
            };
            let font = FontId::proportional(font_size * (1.0 - 0.07 * dist.min(3.0)));
            painter.text(
                egui::pos2(rect.center().x, y),
                Align2::CENTER_CENTER,
                format(values[idx]),
                font,
                color,
            );
        }
        response
    }
}

/// A time of day on drums: hours and minutes (and seconds), in 24 hours or 12 with AM / PM.
pub struct TimeDrum<'a> {
    id_salt: IdSalt,
    time: &'a mut NaiveTime,
    seconds: bool,
    minute_step: usize,
    hour12: bool,
    labels: Option<Arc<Labels>>,
    visible_rows: usize,
}

impl<'a> TimeDrum<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, time: &'a mut NaiveTime) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            time,
            seconds: false,
            minute_step: 1,
            hour12: false,
            labels: None,
            visible_rows: 5,
        }
    }

    /// A drum for the seconds too.
    pub fn seconds(mut self, seconds: bool) -> Self {
        self.seconds = seconds;
        self
    }

    /// Minutes in steps of `step` (5, 15, …).
    pub fn minute_step(mut self, step: usize) -> Self {
        self.minute_step = step.max(1);
        self
    }

    /// Hours 1–12 and an AM / PM drum.
    pub fn hour12(mut self, hour12: bool) -> Self {
        self.hour12 = hour12;
        self
    }

    /// The words for AM and PM.
    pub fn labels(mut self, labels: Arc<Labels>) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn visible_rows(mut self, rows: usize) -> Self {
        self.visible_rows = rows;
        self
    }
}

/// 0–23 as 1–12 and whether it is after noon.
pub(crate) fn to_12h(hour: u32) -> (u32, bool) {
    let h = hour % 12;
    (if h == 0 { 12 } else { h }, hour >= 12)
}

/// 1–12 and after noon as 0–23.
pub(crate) fn from_12h(hour: u32, pm: bool) -> u32 {
    hour % 12 + if pm { 12 } else { 0 }
}

impl Widget for TimeDrum<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let TimeDrum {
            id_salt,
            time,
            seconds,
            minute_step,
            hour12,
            labels,
            visible_rows,
        } = self;
        let labels = labels.unwrap_or_default();
        let (h12, pm) = to_12h(time.hour());
        let mut hour = if hour12 { h12 } else { time.hour() } as i32;
        let mut pm = i32::from(pm);
        let mut minute = time.minute() as i32;
        let mut second = time.second() as i32;
        let two = |v: i32| format!("{v:02}");

        let inner = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let mut changed = false;
            let hours = if hour12 { 1..=12 } else { 0..=23 };
            changed |= ui
                .add(
                    Drum::new((id_salt, "h"), &mut hour, hours)
                        .wrap(true)
                        .format(two)
                        .visible_rows(visible_rows),
                )
                .changed();
            ui.label(":");
            changed |= ui
                .add(
                    Drum::new((id_salt, "m"), &mut minute, 0..=59)
                        .step(minute_step)
                        .wrap(true)
                        .format(two)
                        .visible_rows(visible_rows),
                )
                .changed();
            if seconds {
                ui.label(":");
                changed |= ui
                    .add(
                        Drum::new((id_salt, "s"), &mut second, 0..=59)
                            .wrap(true)
                            .format(two)
                            .visible_rows(visible_rows),
                    )
                    .changed();
            }
            if hour12 {
                let (am, pm_text) = (labels.am.clone(), labels.pm.clone());
                changed |= ui
                    .add(
                        Drum::new((id_salt, "ampm"), &mut pm, 0..=1)
                            .format(move |v| if v == 0 { am.clone() } else { pm_text.clone() })
                            .visible_rows(visible_rows),
                    )
                    .changed();
            }
            changed
        });

        let mut response = inner.response;
        if inner.inner {
            let h = if hour12 {
                from_12h(hour as u32, pm == 1)
            } else {
                hour as u32
            };
            let s = if seconds {
                second as u32
            } else {
                time.second()
            };
            if let Some(t) = NaiveTime::from_hms_opt(h, minute as u32, s) {
                *time = t;
                response.mark_changed();
            }
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest() {
        let fives: Vec<i32> = (0..=59).step_by(5).collect();
        assert_eq!(nearest_index(&fives, 0), 0);
        assert_eq!(nearest_index(&fives, 7), 1);
        assert_eq!(nearest_index(&fives, 8), 2);
        assert_eq!(nearest_index(&fives, 59), 11);
        assert_eq!(nearest_index(&[], 3), 0);
    }

    #[test]
    fn rows() {
        assert_eq!(row_index(-0.4, 60, true), 0);
        assert_eq!(row_index(-0.6, 60, true), 59);
        assert_eq!(row_index(60.2, 60, true), 0);
        assert_eq!(row_index(-3.0, 2, false), 0);
        assert_eq!(row_index(9.0, 2, false), 1);
    }

    #[test]
    fn twelve_hours() {
        assert_eq!(to_12h(0), (12, false));
        assert_eq!(to_12h(11), (11, false));
        assert_eq!(to_12h(12), (12, true));
        assert_eq!(to_12h(23), (11, true));
        for h in 0..24 {
            let (h12, pm) = to_12h(h);
            assert_eq!(from_12h(h12, pm), h);
        }
    }
}
