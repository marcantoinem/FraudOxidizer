use super::{time_axis_formatter, time_x_grid};

pub fn burst_timeline_slot(
    card_id_label: u64,
    card_all: Vec<[f64; 2]>,
    burst_ts_amounts: Vec<[f64; 2]>,
    current_ts: f64,
    current_amount: f64,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(egui::RichText::new(format!("Timeline - card {card_id_label}")).strong());
        let max_amount = card_all
            .iter()
            .map(|p| p[1])
            .fold(current_amount.max(0.0), f64::max)
            .max(1.0);
        let x_axes = vec![
            egui_plot::AxisHints::new_x()
                .label("time")
                .formatter(time_axis_formatter),
        ];
        egui_plot::Plot::new("card_timeline")
            .height(220.0)
            .custom_x_axes(x_axes)
            .x_grid_spacer(time_x_grid)
            .y_axis_label("amount ($)")
            .default_y_bounds(0.0, max_amount * 1.1)
            .y_axis_formatter(|mark, _range| {
                if mark.value < 0.0 {
                    String::new()
                } else {
                    format!("{:.2} $", mark.value)
                }
            })
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
