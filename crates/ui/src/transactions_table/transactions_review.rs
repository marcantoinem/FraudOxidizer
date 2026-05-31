use model::data::human_review_status::HumanReviewStatus;
use model::data::transaction::Transaction;
use model::process::card_statistics::{FraudFactor, HUMAN_REVIEW_SCORE_THRESHOLD_DEFAULT};

use crate::transactions_table::ReviewActionLogEntry;
use crate::transactions_table::country_label;

use super::icons::{ActionIcon, icon_button};
use super::review_command::{ReviewCommand, ReviewUpdate};
use super::review_plots::{
    burst_histogram_slot, burst_timeline_slot, card_amount_deviation_slot,
    category_price_deviation_slot, foreign_trip_table_slot, merchant_ring_slot,
};

pub(crate) fn show_flagged_transactions_review(
    ui: &mut egui::Ui,
    rows: &mut [Transaction],
    review_threshold: &mut f32,
    history: &mut Vec<ReviewActionLogEntry>,
) {
    let max_score = rows
        .iter()
        .map(Transaction::fraud_score)
        .fold(HUMAN_REVIEW_SCORE_THRESHOLD_DEFAULT, f32::max)
        .max(1.0);
    *review_threshold = review_threshold.clamp(0.0, max_score);

    ui.horizontal_wrapped(|ui| {
        ui.label("Human review threshold");
        ui.add(
            egui::Slider::new(review_threshold, 0.0..=max_score)
                .step_by(0.01)
                .show_value(true),
        );
    });
    ui.add_space(8.0);

    let mut flagged_indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (row.fraud_score() >= *review_threshold).then_some(index))
        .collect();
    flagged_indices.sort_by(|a, b| {
        rows[*a]
            .card_id
            .0
            .cmp(&rows[*b].card_id.0)
            .then_with(|| rows[*a].transaction_id.0.cmp(&rows[*b].transaction_id.0))
    });

    if flagged_indices.is_empty() {
        let hidden_count = rows
            .iter()
            .filter(|row| !row.fraud_factors.is_empty() && row.fraud_score() < *review_threshold)
            .count();
        if hidden_count > 0 {
            ui.label(format!(
                "No transactions currently meet the review threshold {:.2}. Lower it to include {} lower-score transactions.",
                *review_threshold, hidden_count
            ));
        } else {
            ui.label("No transactions were flagged by the detector.");
        }
        return;
    }

    ui.label(format!(
        "{} transactions currently require human review at threshold {:.2}.",
        flagged_indices.len(),
        *review_threshold
    ));
    ui.add_space(8.0);

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
    let bulk_approve_shortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::A);
    let bulk_refuse_shortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::R);

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
        push_history_entry(history, rows, &command, "Undo");
        redo_stack.push(command);
    }

    if redo_triggered && let Some(command) = redo_stack.pop() {
        command.apply(rows);
        push_history_entry(history, rows, &command, "Redo");
        undo_stack.push(command);
    }

    let row_index = flagged_indices[cursor];
    let current_card_id = rows[row_index].card_id;
    let current_burst_range = rows[row_index].fraud_factors.iter().find_map(|f| {
        if let FraudFactor::CardTestingBurst {
            burst_start,
            burst_end,
            ..
        } = f
        {
            Some((*burst_start, *burst_end))
        } else {
            None
        }
    });
    let burst_series_indices: Vec<usize> = if let Some((burst_start, burst_end)) =
        current_burst_range
    {
        flagged_indices
                .iter()
                .copied()
                .filter(|&idx| {
                    let r = &rows[idx];
                    r.card_id == current_card_id
                        && r.fraud_factors.iter().any(|f| {
                            matches!(f, FraudFactor::CardTestingBurst { burst_start: s, burst_end: e, .. } if *s == burst_start && *e == burst_end)
                        })
                })
                .collect()
    } else {
        vec![]
    };

    let bulk_approve_triggered = !burst_series_indices.is_empty()
        && ui
            .ctx()
            .input_mut(|i| i.consume_shortcut(&bulk_approve_shortcut));
    let bulk_refuse_triggered = !burst_series_indices.is_empty()
        && ui
            .ctx()
            .input_mut(|i| i.consume_shortcut(&bulk_refuse_shortcut));

    let row = &rows[row_index];
    let mut requested_status: Option<HumanReviewStatus> = None;
    let mut requested_bulk_status: Option<HumanReviewStatus> = None;
    let current_category_deviation = row.fraud_factors.iter().find_map(|f| {
        if let FraudFactor::CategoryPriceDeviation {
            category,
            amount,
            average_amount,
            std_deviation,
            z_score,
            ..
        } = f
        {
            Some((
                *category,
                *amount,
                *average_amount,
                *std_deviation,
                *z_score,
            ))
        } else {
            None
        }
    });
    let current_card_deviation = row.fraud_factors.iter().find_map(|f| {
        if let FraudFactor::CardAmountDeviation {
            card_id,
            amount,
            average_amount,
            std_deviation,
            z_score,
            ..
        } = f
        {
            Some((*card_id, *amount, *average_amount, *std_deviation, *z_score))
        } else {
            None
        }
    });
    let current_merchant_ring = row.fraud_factors.iter().find_map(|f| {
        if let FraudFactor::MerchantRing {
            merchant_name,
            amount,
            merchant_median,
            ratio,
            outlier_count,
            distinct_card_count,
        } = f
        {
            Some((
                merchant_name.clone(),
                *amount,
                *merchant_median,
                *ratio,
                *outlier_count,
                *distinct_card_count,
            ))
        } else {
            None
        }
    });

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
                if row.fraud_factors.len() > 1 {
                    ui.colored_label(
                        egui::Color32::from_rgb(250, 235, 175),
                        format!("combined signals x{}", row.fraud_factors.len()),
                    );
                }
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
                    }

                    let fraud_clicked =
                        icon_button(ui, "Mark fraud", ActionIcon::Fraud, true).clicked();
                    if fraud_clicked || fraud_triggered {
                        requested_status = Some(HumanReviewStatus::TruePositive);
                    }

                    if !burst_series_indices.is_empty() {
                        ui.separator();
                        let bulk_approve_clicked =
                            icon_button(ui, "Approve burst", ActionIcon::Approve, true).clicked();
                        if bulk_approve_clicked || bulk_approve_triggered {
                            requested_bulk_status = Some(HumanReviewStatus::FalsePositive);
                        }

                        let bulk_refuse_clicked =
                            icon_button(ui, "Mark burst fraud", ActionIcon::Fraud, true).clicked();
                        if bulk_refuse_clicked || bulk_refuse_triggered {
                            requested_bulk_status = Some(HumanReviewStatus::TruePositive);
                        }
                    }
                });
            });

            let current_ts = row.timestamp.timestamp() as f64;
            let current_amount = row.amount;
            let card_id_label = current_card_id.0;

            let mut plot_slots: Vec<Box<dyn FnOnce(&mut egui::Ui)>> = Vec::new();

            if current_burst_range.is_some() {
                let card_amounts: Vec<f64> = rows
                    .iter()
                    .filter(|r| r.card_id == current_card_id)
                    .map(|r| r.amount)
                    .collect();
                let card_all: Vec<[f64; 2]> = rows
                    .iter()
                    .filter(|r| r.card_id == current_card_id)
                    .map(|r| [r.timestamp.timestamp() as f64, r.amount])
                    .collect();
                let burst_ts_amounts: Vec<[f64; 2]> = burst_series_indices
                    .iter()
                    .map(|&idx| [rows[idx].timestamp.timestamp() as f64, rows[idx].amount])
                    .collect();

                plot_slots.push(burst_histogram_slot(
                    card_id_label,
                    card_amounts,
                    current_amount,
                ));
                plot_slots.push(burst_timeline_slot(
                    card_id_label,
                    card_all,
                    burst_ts_amounts,
                    current_ts,
                    current_amount,
                ));
            }

            if let Some((card_id, amount, average_amount, std_deviation, z_score)) =
                current_card_deviation
            {
                let card_all: Vec<[f64; 2]> = rows
                    .iter()
                    .filter(|r| r.card_id.0 == card_id)
                    .map(|r| [r.timestamp.timestamp() as f64, r.amount])
                    .collect();
                plot_slots.push(card_amount_deviation_slot(
                    card_id_label,
                    card_all,
                    current_ts,
                    amount,
                    average_amount,
                    std_deviation,
                    z_score,
                ));
            }

            if let Some((category, amount, average_amount, std_deviation, z_score)) =
                current_category_deviation
            {
                let category_label = format!("{:?}", category);
                let category_all: Vec<[f64; 2]> = rows
                    .iter()
                    .filter(|r| r.merchant_category == category)
                    .map(|r| [r.timestamp.timestamp() as f64, r.amount])
                    .collect();
                plot_slots.push(category_price_deviation_slot(
                    category_label,
                    category_all,
                    current_ts,
                    amount,
                    average_amount,
                    std_deviation,
                    z_score,
                ));
            }

            if let Some((
                merchant_name,
                amount,
                merchant_median,
                ratio,
                outlier_count,
                distinct_card_count,
            )) = current_merchant_ring.clone()
            {
                let merchant_rows: Vec<&Transaction> = rows
                    .iter()
                    .filter(|r| r.merchant_name == merchant_name)
                    .collect();
                let merchant_points: Vec<[f64; 2]> = merchant_rows
                    .iter()
                    .enumerate()
                    .map(|(idx, r)| [idx as f64 + 1.0, r.amount])
                    .collect();
                let current_x = merchant_rows
                    .iter()
                    .position(|r| r.transaction_id == row.transaction_id)
                    .map(|idx| idx as f64 + 1.0)
                    .unwrap_or(1.0);

                plot_slots.push(merchant_ring_slot(
                    merchant_name,
                    merchant_points,
                    current_x,
                    amount,
                    merchant_median,
                    ratio,
                    outlier_count,
                    distinct_card_count,
                ));
            }

            let has_foreign_trip = row
                .fraud_factors
                .iter()
                .any(|f| matches!(f, FraudFactor::ForeignCountryTrip { .. }));
            if has_foreign_trip {
                let mut trip_rows: Vec<(i64, String, f64, String, bool)> = rows
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.card_id == current_card_id)
                    .map(|(idx, r)| {
                        let ts = r.timestamp.timestamp();
                        let time_str = r.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                        let country = country_label(r.cardholder_country);
                        (ts, time_str, r.amount, country, idx == row_index)
                    })
                    .collect();
                trip_rows.sort_by_key(|(ts, _, _, _, _)| *ts);
                plot_slots.push(foreign_trip_table_slot(card_id_label, trip_rows));
            }

            if !plot_slots.is_empty() {
                ui.add_space(8.0);
                let mut iter = plot_slots.into_iter().peekable();
                while iter.peek().is_some() {
                    let a = iter.next();
                    let b = iter.next();
                    ui.columns(2, |cols| {
                        if let Some(slot) = a {
                            slot(&mut cols[0]);
                        }
                        if let Some(slot) = b {
                            slot(&mut cols[1]);
                        }
                    });
                }
            }

            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                shortcut_legend_item(ui, &redo_shortcut, "redo");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &undo_shortcut, "undo");
                if !burst_series_indices.is_empty() {
                    ui.add_space(10.0);
                    shortcut_legend_item(ui, &bulk_refuse_shortcut, "mark burst fraud");
                    ui.add_space(10.0);
                    shortcut_legend_item(ui, &bulk_approve_shortcut, "approve burst");
                }
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
        let before_status = rows[row_index].human_review_status;
        let before_factors = rows[row_index].fraud_factors.clone();
        let after_factors = deactivate_card_testing_burst_factors(&before_factors);
        if before_status != after || before_factors != after_factors {
            let command = ReviewCommand::SetHumanReviewStatus {
                update: ReviewUpdate {
                    transaction_index: row_index,
                    before_status,
                    after_status: after,
                    before_factors,
                    after_factors,
                },
            };
            command.apply(rows);
            let label = if matches!(after, HumanReviewStatus::FalsePositive) {
                "Approve"
            } else {
                "Mark fraud"
            };
            push_history_entry(history, rows, &command, label);
            undo_stack.push(command);
            redo_stack.clear();
        }
    }

    if let Some(after) = requested_bulk_status {
        let updates: Vec<ReviewUpdate> = burst_series_indices
            .iter()
            .copied()
            .filter_map(|transaction_index| {
                let before_status = rows[transaction_index].human_review_status;
                let before_factors = rows[transaction_index].fraud_factors.clone();
                let after_factors = deactivate_card_testing_burst_factors(&before_factors);
                (before_status != after || before_factors != after_factors).then_some(
                    ReviewUpdate {
                        transaction_index,
                        before_status,
                        after_status: after,
                        before_factors,
                        after_factors,
                    },
                )
            })
            .collect();

        if !updates.is_empty() {
            let command = ReviewCommand::BatchSetHumanReviewStatus { updates };
            command.apply(rows);
            let label = if matches!(after, HumanReviewStatus::FalsePositive) {
                "Approve burst"
            } else {
                "Mark burst fraud"
            };
            push_history_entry(history, rows, &command, label);
            undo_stack.push(command);
            redo_stack.clear();
        }
    }

    ui.ctx().data_mut(|d| {
        d.insert_persisted(cursor_id, cursor);
        d.insert_temp(undo_stack_id, undo_stack);
        d.insert_temp(redo_stack_id, redo_stack);
    });
}

fn deactivate_card_testing_burst_factors(factors: &[FraudFactor]) -> Vec<FraudFactor> {
    factors
        .iter()
        .map(|factor| match factor {
            FraudFactor::CardTestingBurst {
                transaction_count,
                burst_start,
                burst_end,
                max_amount,
                max_gap,
            } => FraudFactor::InactiveCardTestingBurst {
                transaction_count: *transaction_count,
                burst_start: *burst_start,
                burst_end: *burst_end,
                max_amount: *max_amount,
                max_gap: *max_gap,
            },
            _ => factor.clone(),
        })
        .collect()
}

fn push_history_entry(
    history: &mut Vec<ReviewActionLogEntry>,
    rows: &[Transaction],
    command: &ReviewCommand,
    label: &str,
) {
    let updates: Vec<&ReviewUpdate> = match command {
        ReviewCommand::SetHumanReviewStatus { update } => vec![update],
        ReviewCommand::BatchSetHumanReviewStatus { updates } => updates.iter().collect(),
    };

    if updates.is_empty() {
        return;
    }

    let mut tx_ids = Vec::new();
    let mut status_transitions = Vec::new();
    for update in &updates {
        if let Some(row) = rows.get(update.transaction_index) {
            tx_ids.push(row.transaction_id.0.to_string());
        }
        status_transitions.push(format!(
            "{} -> {}",
            status_label(update.before_status),
            status_label(update.after_status)
        ));
    }

    let mut summary = if tx_ids.len() <= 5 {
        format!("tx {}", tx_ids.join(", "))
    } else {
        format!("tx {} + {} more", tx_ids[..5].join(", "), tx_ids.len() - 5)
    };
    if let Some(first_transition) = status_transitions.first() {
        summary.push_str(&format!(" · {}", first_transition));
    }

    history.push(ReviewActionLogEntry {
        label: label.to_owned(),
        changed_count: updates.len(),
        summary,
        timestamp: chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
    });
}

fn status_label(status: HumanReviewStatus) -> &'static str {
    match status {
        HumanReviewStatus::NotNeeded => "not-needed",
        HumanReviewStatus::NeedCheck => "need-check",
        HumanReviewStatus::FalsePositive => "approved",
        HumanReviewStatus::TruePositive => "fraud",
    }
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
