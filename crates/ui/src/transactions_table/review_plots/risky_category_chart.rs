pub fn risky_category_slot(category_label: String, weight: f32) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        ui.label(egui::RichText::new(format!("Risky category signal - {category_label}")).strong());
        ui.label("Category pricing companion triggers at +3 std dev.");

        let category_pricing_base_weight =
            model::process::card_statistics::CATEGORY_PRICE_DEVIATION_BASE_WEIGHT as f64;

        let bars = vec![
            egui_plot::Bar::new(0.0, weight as f64)
                .width(0.6)
                .name("risk weight")
                .fill(egui::Color32::from_rgb(220, 180, 80)),
            egui_plot::Bar::new(1.0, category_pricing_base_weight)
                .width(0.6)
                .name("+3 std dev base weight")
                .fill(egui::Color32::from_rgb(100, 210, 255)),
        ];

        egui_plot::Plot::new(format!("risky_category_{category_label}"))
            .height(220.0)
            .y_axis_label("score")
            .default_y_bounds(0.0, 1.0)
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(egui_plot::BarChart::new("risk", bars));
            });
    })
}
