use std::ops::RangeInclusive;

mod burst_histogram;
mod burst_timeline;
mod card_amount_deviation_chart;
mod category_price_deviation_chart;
mod foreign_trip_table;
mod merchant_ring_chart;

pub(super) use burst_histogram::burst_histogram_slot;
pub(super) use burst_timeline::burst_timeline_slot;
pub(super) use card_amount_deviation_chart::card_amount_deviation_slot;
pub(super) use category_price_deviation_chart::category_price_deviation_slot;
pub(super) use foreign_trip_table::foreign_trip_table_slot;
pub(super) use merchant_ring_chart::merchant_ring_slot;

const SECS_PER_DAY: f64 = 24.0 * 60.0 * 60.0;
const SECS_PER_WEEK: f64 = 7.0 * SECS_PER_DAY;
const SECS_PER_HOUR: f64 = 60.0 * 60.0;
const SECS_PER_5_MIN: f64 = 5.0 * 60.0;

fn is_approx_integer(val: f64) -> bool {
    val.fract().abs() < 1e-6
}

fn is_day_boundary(ts: f64) -> bool {
    is_approx_integer(ts / SECS_PER_DAY)
}

fn is_hour_boundary(ts: f64) -> bool {
    is_approx_integer(ts / SECS_PER_HOUR)
}

fn is_five_min_boundary(ts: f64) -> bool {
    is_approx_integer(ts / SECS_PER_5_MIN)
}

pub(super) fn time_x_grid(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let mut marks = Vec::new();
    let (min, max) = input.bounds;
    let span = (max - min).max(0.0);

    let mut push_marks = |step: f64| {
        let mut v = (min / step).ceil() * step;
        while v <= max {
            marks.push(egui_plot::GridMark {
                value: v,
                step_size: step,
            });
            v += step;
        }
    };

    if span > 10.0 * SECS_PER_DAY {
        push_marks(SECS_PER_WEEK);
        if span <= 28.0 * SECS_PER_DAY {
            push_marks(SECS_PER_DAY);
        }
    } else {
        push_marks(SECS_PER_DAY);
        if span <= 3.0 * SECS_PER_DAY {
            push_marks(SECS_PER_HOUR);
        }
        if span <= 6.0 * SECS_PER_HOUR {
            push_marks(SECS_PER_5_MIN);
        }
    }

    marks
}

pub(super) fn time_axis_formatter(
    mark: egui_plot::GridMark,
    range: &RangeInclusive<f64>,
) -> String {
    let ts = mark.value;
    let Some(dt) = chrono::DateTime::from_timestamp(ts as i64, 0) else {
        return String::new();
    };
    let span = (*range.end() - *range.start()).max(0.0);

    if span > 10.0 * SECS_PER_DAY {
        use chrono::Datelike;
        return format!("W{:02} {}", dt.iso_week().week(), dt.format("%m-%d"));
    }

    if span > 3.0 * SECS_PER_DAY {
        return dt.format("%m-%d").to_string();
    }

    if is_day_boundary(ts) {
        dt.format("%m-%d").to_string()
    } else if is_hour_boundary(ts) || is_five_min_boundary(ts) {
        dt.format("%H:%M").to_string()
    } else {
        String::new()
    }
}
