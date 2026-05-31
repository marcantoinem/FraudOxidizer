use super::{time_axis_formatter, time_x_grid};

pub fn category_price_deviation_slot(
    category_label: String,
    category_all: Vec<[f64; 2]>,
    current_ts: f64,
    current_amount: f64,
    average_amount: f64,
    std_deviation: f64,
    z_score: f64,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(
            egui::RichText::new(format!("Category spend profile - {category_label}")).strong(),
        );
        ui.label(format!(
            "average {:.2} $, std dev {:.2} $, z-score {:.2}",
            average_amount, std_deviation, z_score
        ));

        let max_amount = category_all
            .iter()
            .map(|point| point[1])
            .fold(current_amount.max(average_amount), f64::max)
            .max(1.0);
        let limit_sigma = model::process::card_statistics::CATEGORY_PRICE_DEVIATION_MIN_Z_SCORE;
        let trigger_amount = average_amount + std_deviation * limit_sigma;
        let y_max = (max_amount.max(trigger_amount)) * 1.1;
        let x_axes = vec![
            egui_plot::AxisHints::new_x()
                .label("time")
                .formatter(time_axis_formatter),
        ];

        egui_plot::Plot::new(format!("category_price_deviation_{category_label}"))
            .height(220.0)
            .custom_x_axes(x_axes)
            .x_grid_spacer(time_x_grid)
            .y_axis_label("amount ($)")
            .default_y_bounds(0.0, y_max)
            .show(ui, |plot_ui| {
                plot_ui.points(
                    egui_plot::Points::new(
                        "category transactions",
                        egui_plot::PlotPoints::new(category_all),
                    )
                    .radius(3.0)
                    .color(egui::Color32::from_gray(130)),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("mean", average_amount)
                        .color(egui::Color32::from_rgb(120, 180, 120))
                        .width(2.0),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("+1 std dev", average_amount + std_deviation)
                        .color(egui::Color32::from_rgb(160, 210, 110))
                        .width(1.5),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("+2 std dev", average_amount + std_deviation * 2.0)
                        .color(egui::Color32::from_rgb(210, 170, 90))
                        .width(1.5),
                );
                plot_ui.hline(
                    egui_plot::HLine::new(
                        "+3 std dev limit",
                        average_amount + std_deviation * limit_sigma,
                    )
                    .color(egui::Color32::from_rgb(225, 90, 90))
                    .width(1.6),
                );
                plot_ui.points(
                    egui_plot::Points::new(
                        "current",
                        egui_plot::PlotPoints::new(vec![[current_ts, current_amount]]),
                    )
                    .radius(5.0)
                    .color(egui::Color32::from_rgb(100, 210, 255)),
                );
            });
    })
}
