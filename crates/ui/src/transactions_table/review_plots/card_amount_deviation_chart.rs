pub fn card_amount_deviation_slot(
    card_id_label: u64,
    card_all: Vec<[f64; 2]>,
    current_ts: f64,
    current_amount: f64,
    average_amount: f64,
    std_deviation: f64,
    z_score: f64,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(egui::RichText::new(format!("Card spend profile - card {card_id_label}")).strong());
        ui.label(format!("average {:.2} $, std dev {:.2} $, z-score {:.2}", average_amount, std_deviation, z_score));

        let max_amount = card_all
            .iter()
            .map(|point| point[1])
            .fold(current_amount.max(average_amount), f64::max)
            .max(1.0);
        let y_max = (max_amount.max(average_amount + std_deviation * 2.0)) * 1.1;
        let y_min = 0.0;

        egui_plot::Plot::new(format!("card_amount_deviation_{card_id_label}"))
            .height(220.0)
            .y_axis_label("amount ($)")
            .default_y_bounds(y_min, y_max)
            .show(ui, |plot_ui| {
                plot_ui.points(
                    egui_plot::Points::new(
                        "card transactions",
                        egui_plot::PlotPoints::new(card_all),
                    )
                    .radius(3.0)
                    .color(egui::Color32::from_gray(130)),
                );
                plot_ui.vline(
                    egui_plot::VLine::new("mean", 0.0)
                        .color(egui::Color32::from_rgb(120, 180, 120))
                        .width(0.0),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("mean", average_amount)
                        .color(egui::Color32::from_rgb(120, 180, 120))
                        .width(2.0),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("+1 std dev", average_amount + std_deviation)
                        .color(egui::Color32::from_rgb(220, 180, 80))
                        .width(1.5),
                );
                plot_ui.hline(
                    egui_plot::HLine::new("+2 std dev", average_amount + std_deviation * 2.0)
                        .color(egui::Color32::from_rgb(225, 90, 90))
                        .width(1.5),
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