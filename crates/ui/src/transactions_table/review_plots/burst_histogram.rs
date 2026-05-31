pub fn burst_histogram_slot(
    card_id_label: u64,
    card_amounts: Vec<f64>,
    current_amount: f64,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(
            egui::RichText::new(format!("Amount distribution - card {card_id_label}")).strong(),
        );
        let bin_size = 10.0_f64;
        let max_amount = card_amounts.iter().cloned().fold(0.0_f64, f64::max);
        let num_bins = ((max_amount / bin_size).ceil() as usize).max(1);
        let mut counts = vec![0u64; num_bins];
        for &amount in &card_amounts {
            let bin = ((amount / bin_size).floor().max(0.0) as usize).min(num_bins - 1);
            counts[bin] += 1;
        }
        let bars: Vec<egui_plot::Bar> = counts
            .iter()
            .enumerate()
            .map(|(i, &count)| {
                egui_plot::Bar::new(i as f64 * bin_size + bin_size / 2.0, count as f64)
                    .width(bin_size * 0.9)
            })
            .collect();
        egui_plot::Plot::new("card_amount_histogram")
            .height(280.0)
            .y_axis_label("count")
            .x_axis_label("amount ($)")
            .x_axis_formatter(|mark, _range| {
                if mark.value < 0.0 {
                    String::new()
                } else {
                    format!("{:.2} $", mark.value)
                }
            })
            .y_axis_formatter(|mark, _range| {
                if mark.value < 0.0 {
                    String::new()
                } else {
                    format!("{:.0}", mark.value)
                }
            })
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(egui_plot::BarChart::new("count", bars));
                plot_ui.vline(
                    egui_plot::VLine::new("current", current_amount)
                        .color(egui::Color32::from_rgb(100, 210, 255))
                        .width(2.0),
                );
            });
    })
}
