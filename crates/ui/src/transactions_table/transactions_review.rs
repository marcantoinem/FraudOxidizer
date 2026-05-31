use model::data::human_review_status::HumanReviewStatus;
use model::data::transaction::Transaction;

use super::icons::{ActionIcon, icon_button};
use super::review_command::ReviewCommand;

pub(crate) fn show_flagged_transactions_review(ui: &mut egui::Ui, rows: &mut [Transaction]) {
    let flagged_indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (!row.fraud_factors.is_empty()).then_some(index))
        .collect();

    if flagged_indices.is_empty() {
        ui.label("No transactions were flagged by the detector.");
        return;
    }

    let cursor_id = ui.make_persistent_id("flagged_review_carousel_cursor");
    let undo_stack_id = ui.make_persistent_id("flagged_review_undo_stack");
    let redo_stack_id = ui.make_persistent_id("flagged_review_redo_stack");
    let mut cursor = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<usize>(cursor_id))
        .unwrap_or(0);
    let mut undo_stack = ui
        .ctx()
        .data_mut(|d| d.get_temp::<Vec<ReviewCommand>>(undo_stack_id))
        .unwrap_or_default();
    let mut redo_stack = ui
        .ctx()
        .data_mut(|d| d.get_temp::<Vec<ReviewCommand>>(redo_stack_id))
        .unwrap_or_default();

    let item_count = flagged_indices.len();
    if cursor >= item_count {
        cursor = item_count - 1;
    }

    let previous_shortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowLeft);
    let next_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowRight);
    let approve_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::A);
    let fraud_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F);
    let undo_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z);
    let redo_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y);

    let previous_triggered = ui
        .ctx()
        .input_mut(|i| i.consume_shortcut(&previous_shortcut));
    let next_triggered = ui.ctx().input_mut(|i| i.consume_shortcut(&next_shortcut));
    let approve_triggered = ui
        .ctx()
        .input_mut(|i| i.consume_shortcut(&approve_shortcut));
    let fraud_triggered = ui.ctx().input_mut(|i| i.consume_shortcut(&fraud_shortcut));
    let undo_triggered = ui.ctx().input_mut(|i| i.consume_shortcut(&undo_shortcut));
    let redo_triggered = ui.ctx().input_mut(|i| i.consume_shortcut(&redo_shortcut));

    if previous_triggered {
        cursor = if cursor == 0 {
            item_count - 1
        } else {
            cursor - 1
        };
    }
    if next_triggered {
        cursor = (cursor + 1) % item_count;
    }

    if undo_triggered && let Some(command) = undo_stack.pop() {
        command.undo(rows);
        if let Some(position) = flagged_indices
            .iter()
            .position(|&idx| idx == command.transaction_index())
        {
            cursor = position;
        }
        redo_stack.push(command);
    }

    if redo_triggered && let Some(command) = redo_stack.pop() {
        command.apply(rows);
        if let Some(position) = flagged_indices
            .iter()
            .position(|&idx| idx == command.transaction_index())
        {
            cursor = position;
        }
        undo_stack.push(command);
    }

    let row_index = flagged_indices[cursor];
    let row = &rows[row_index];
    let mut requested_status: Option<HumanReviewStatus> = None;
    let mut advance_after_action = false;

    ui.horizontal(|ui| {
        let previous_clicked = icon_button(ui, "Previous", ActionIcon::Previous, true).clicked();
        if previous_clicked {
            cursor = if cursor == 0 {
                item_count - 1
            } else {
                cursor - 1
            };
        }
        ui.strong(format!("{} / {}", cursor + 1, item_count));
        let next_clicked = icon_button(ui, "Next", ActionIcon::Next, true).clicked();
        if next_clicked {
            cursor = (cursor + 1) % item_count;
        }
    });

    ui.add_space(8.0);

    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading(format!(
                    "tx_{} · card_{}",
                    row.transaction_id.0, row.card_id.0
                ));
                ui.label(format!("{:.2} $", row.amount));
                ui.label(format!("{}", row.timestamp.format("%Y-%m-%d %H:%M:%S")));
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Merchant: {}", row.merchant_name));
                ui.label(format!("Category: {:?}", row.merchant_category));
                ui.label(format!("Channel: {:?}", row.channel));
                ui.label(format!(
                    "Country: {} -> {}",
                    row.cardholder_country.0.alpha2(),
                    row.merchant_country.0.alpha2()
                ));
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if matches!(row.human_review_status, HumanReviewStatus::FalsePositive) {
                    ui.colored_label(egui::Color32::from_rgb(120, 180, 120), "Reviewed clean");
                } else if matches!(row.human_review_status, HumanReviewStatus::TruePositive) {
                    ui.colored_label(egui::Color32::from_rgb(225, 90, 90), "Reviewed fraud");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(250, 235, 175), "Needs review");
                }
                ui.label(format!("score {:.2}", row.fraud_score()));
            });

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for factor in &row.fraud_factors {
                    super::render_reason_chip(ui, factor);
                }
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.scope(|ui| {
                    ui.spacing_mut().button_padding = egui::vec2(12.0, 8.0);
                    ui.spacing_mut().interact_size.y = 36.0;

                    let approve_clicked =
                        icon_button(ui, "Approve", ActionIcon::Approve, true).clicked();
                    if approve_clicked || approve_triggered {
                        requested_status = Some(HumanReviewStatus::FalsePositive);
                        advance_after_action = true;
                    }

                    let fraud_clicked =
                        icon_button(ui, "Mark fraud", ActionIcon::Fraud, true).clicked();
                    if fraud_clicked || fraud_triggered {
                        requested_status = Some(HumanReviewStatus::TruePositive);
                        advance_after_action = true;
                    }
                });
            });

            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                shortcut_legend_item(ui, &redo_shortcut, "redo");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &undo_shortcut, "undo");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &fraud_shortcut, "mark fraud");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &approve_shortcut, "approve");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &next_shortcut, "next");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &previous_shortcut, "previous");
            });
        });

    if let Some(after) = requested_status {
        let before = rows[row_index].human_review_status;
        if before != after {
            let command = ReviewCommand::SetHumanReviewStatus {
                transaction_index: row_index,
                before,
                after,
            };
            command.apply(rows);
            undo_stack.push(command);
            redo_stack.clear();

            if advance_after_action && item_count > 1 {
                cursor = (cursor + 1) % item_count;
            }
        }
    }

    ui.ctx().data_mut(|d| {
        d.insert_persisted(cursor_id, cursor);
        d.insert_temp(undo_stack_id, undo_stack);
        d.insert_temp(redo_stack_id, redo_stack);
    });
}

fn shortcut_ui(ui: &mut egui::Ui, shortcut: &egui::KeyboardShortcut) {
    let text = ui.ctx().format_shortcut(shortcut);
    let body_height = ui.text_style_height(&egui::TextStyle::Body);
    let dark_mode = ui.visuals().dark_mode;
    let key_fill = ui.visuals().widgets.inactive.bg_fill.linear_multiply(1.05);
    let key_stroke = ui
        .visuals()
        .widgets
        .inactive
        .bg_stroke
        .color
        .linear_multiply(1.05);
    let key_text = ui.visuals().widgets.inactive.fg_stroke.color;

    let font_id = egui::FontId::monospace((body_height - 2.0).max(10.0));
    let galley = ui.painter().layout_no_wrap(text, font_id.clone(), key_text);
    let horizontal_padding = 6.0;
    let vertical_padding = 2.0;
    let desired_size = egui::vec2(
        (galley.size().x + horizontal_padding * 2.0).max(body_height + 6.0),
        (galley.size().y + vertical_padding * 2.0).max(body_height + 2.0),
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        key_fill,
        egui::Stroke::new(1.0, key_stroke),
        egui::StrokeKind::Inside,
    );

    let text_pos = egui::pos2(
        rect.center().x - galley.size().x / 2.0,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, key_text);

    let rect = response.rect.shrink(1.0);
    let highlight = key_fill.gamma_multiply(if dark_mode { 1.18 } else { 1.05 });
    let shadow = key_fill.gamma_multiply(if dark_mode { 0.55 } else { 0.88 });

    ui.painter().hline(
        rect.x_range(),
        rect.top() + 0.5,
        egui::Stroke::new(1.0, highlight),
    );
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, shadow),
    );
}

fn shortcut_legend_item(ui: &mut egui::Ui, shortcut: &egui::KeyboardShortcut, label: &str) {
    ui.horizontal(|ui| {
        shortcut_ui(ui, shortcut);
        ui.label(label);
    });
}
