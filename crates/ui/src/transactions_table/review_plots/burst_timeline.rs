pub fn burst_timeline_slot(
    card_id_label: u64,
    card_all: Vec<[f64; 2]>,
    burst_ts_amounts: Vec<[f64; 2]>,
    current_ts: f64,
    current_amount: f64,
    current_time_str: String,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(egui::RichText::new(format!("Timeline - card {card_id_label}")).strong());
        egui_plot::Plot::new("card_timeline")
            .height(280.0)
            .x_axis_formatter(|mark, _range| {
                chrono::DateTime::from_timestamp(mark.value as i64, 0)
                    .map(|d: chrono::DateTime<chrono::Utc>| d.format("%m-%d %H:%M").to_string())
                    .unwrap_or_default()
            })
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
                    for point in &burst_ts_amounts {
                        let time = chrono::DateTime::from_timestamp(point[0] as i64, 0)
                            .map(|d: chrono::DateTime<chrono::Utc>| d.format("%H:%M").to_string())
                            .unwrap_or_default();
                        plot_ui.text(
                            egui_plot::Text::new(
                                "",
                                egui_plot::PlotPoint::new(point[0], point[1]),
                                format!("burst {:.2} $ {}", point[1], time),
                            )
                            .anchor(egui::Align2::LEFT_BOTTOM)
                            .color(egui::Color32::from_rgb(255, 190, 110)),
                        );
                    }
                }
                plot_ui.points(
                    egui_plot::Points::new(
                        "current",
                        egui_plot::PlotPoints::new(vec![[current_ts, current_amount]]),
                    )
                    .radius(4.0)
                    .color(egui::Color32::from_rgb(100, 210, 255)),
                );
                plot_ui.text(
                    egui_plot::Text::new(
                        "",
                        egui_plot::PlotPoint::new(current_ts, current_amount),
                        format!("{:.2} $ {}", current_amount, current_time_str),
                    )
                    .anchor(egui::Align2::LEFT_BOTTOM)
                    .color(egui::Color32::from_rgb(100, 210, 255)),
                );
            });
    })
}
