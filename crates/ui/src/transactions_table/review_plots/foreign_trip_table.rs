pub fn foreign_trip_table_slot(
    card_id_label: u64,
    home_country: String,
    trip_rows: Vec<(i64, String, f64, String, bool)>,
) -> Box<dyn FnOnce(&mut egui::Ui)> {
    Box::new(move |ui: &mut egui::Ui| {
        let current_pos = trip_rows
            .iter()
            .position(|(_, _, _, _, is_current)| *is_current);
        let window: &[(i64, String, f64, String, bool)] = if let Some(pos) = current_pos {
            let start = pos.saturating_sub(4);
            let end = (pos + 5).min(trip_rows.len());
            &trip_rows[start..end]
        } else {
            &trip_rows
        };

        let before_count = current_pos.map(|p| p.saturating_sub(4)).unwrap_or(0);
        let after_omitted = current_pos
            .map(|p| trip_rows.len().saturating_sub(p + 5))
            .unwrap_or(0);

        ui.label(
            egui::RichText::new(format!(
                "Transactions - card {card_id_label} (home: {home_country})"
            ))
            .strong(),
        );
        ui.add_space(4.0);
        egui::Grid::new("foreign_trip_table")
            .striped(true)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.strong("Time");
                ui.strong("Amount");
                ui.strong("Country");
                ui.end_row();
                if before_count > 0 {
                    ui.weak(format!("... {} earlier", before_count));
                    ui.label("");
                    ui.label("");
                    ui.end_row();
                }
                for (_, time_str, amount, country, is_current) in window {
                    let color = if *is_current {
                        egui::Color32::from_rgb(100, 210, 255)
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(color, time_str);
                    ui.colored_label(color, format!("{:.2} $", amount));
                    ui.colored_label(color, country);
                    ui.end_row();
                }
                if after_omitted > 0 {
                    ui.weak(format!("... {} later", after_omitted));
                    ui.label("");
                    ui.label("");
                    ui.end_row();
                }
            });
    })
}
