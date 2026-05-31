pub fn merchant_ring_slot(
    merchant_name: String,
    merchant_points: Vec<[f64; 2]>,
    current_x: f64,
    current_amount: f64,
    merchant_median: f64,
    ratio: f64,
    outlier_count: usize,
    distinct_card_count: usize,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(egui::RichText::new(format!("Merchant ring - {merchant_name}")).strong());
        ui.label(format!(
            "median {:.2} $, current {:.2} $ ({:.1}x), outliers {} across {} cards",
            merchant_median, current_amount, ratio, outlier_count, distinct_card_count
        ));

        let limit_amount =
            merchant_median * model::process::card_statistics::MERCHANT_RING_MULTIPLIER;
        let max_amount = merchant_points
            .iter()
            .map(|point| point[1])
            .fold(current_amount.max(limit_amount).max(1.0), f64::max);

        egui_plot::Plot::new(format!("merchant_ring_{merchant_name}"))
            .height(220.0)
            .x_axis_label("transaction #")
            .y_axis_label("amount ($)")
            .default_y_bounds(0.0, max_amount * 1.1)
            .show(ui, |plot_ui| {
                plot_ui.points(
                    egui_plot::Points::new(
                        "merchant transactions",
                        egui_plot::PlotPoints::new(merchant_points),
                    )
                    .radius(3.5)
                    .color(egui::Color32::from_gray(130)),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("merchant median", merchant_median)
                        .color(egui::Color32::from_rgb(120, 180, 120))
                        .width(1.8),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("5x median threshold", limit_amount)
                        .color(egui::Color32::from_rgb(225, 90, 90))
                        .width(1.8),
                );
                plot_ui.points(
                    egui_plot::Points::new(
                        "current",
                        egui_plot::PlotPoints::new(vec![[current_x, current_amount]]),
                    )
                    .radius(5.0)
                    .color(egui::Color32::from_rgb(100, 210, 255)),
                );
            });
    })
}
