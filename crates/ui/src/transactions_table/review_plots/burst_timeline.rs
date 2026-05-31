use std::ops::RangeInclusive;

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

fn x_grid(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
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

fn time_formatter(mark: egui_plot::GridMark, range: &RangeInclusive<f64>) -> String {
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

pub fn burst_timeline_slot(
    card_id_label: u64,
    card_all: Vec<[f64; 2]>,
    burst_ts_amounts: Vec<[f64; 2]>,
    current_ts: f64,
    current_amount: f64,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(egui::RichText::new(format!("Timeline - card {card_id_label}")).strong());
        let x_axes = vec![
            egui_plot::AxisHints::new_x()
                .label("time")
                .formatter(time_formatter),
        ];
        egui_plot::Plot::new("card_timeline")
            .height(280.0)
            .custom_x_axes(x_axes)
            .x_grid_spacer(x_grid)
            .y_axis_label("amount ($)")
            .y_axis_formatter(|mark, _range| format!("{:.2} $", mark.value))
            .show(ui, |plot_ui| {
                plot_ui.points(
                    egui_plot::Points::new(
                        "other card transactions",
                        egui_plot::PlotPoints::new(card_all),
                    )
                    .radius(3.0)
                    .color(egui::Color32::from_gray(130)),
                );
                if !burst_ts_amounts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new(
                            "burst transactions",
                            egui_plot::PlotPoints::new(burst_ts_amounts.clone()),
                        )
                        .radius(4.0)
                        .color(egui::Color32::from_rgb(255, 160, 50)),
                    );
                }
                plot_ui.points(
                    egui_plot::Points::new(
                        "current",
                        egui_plot::PlotPoints::new(vec![[current_ts, current_amount]]),
                    )
                    .radius(4.0)
                    .color(egui::Color32::from_rgb(100, 210, 255)),
                );
            });
    })
}
