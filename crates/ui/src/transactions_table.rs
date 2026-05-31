use model::data::human_review_status::HumanReviewStatus;
use model::data::transaction::Transaction;
use model::process::card_statistics::FraudFactor;
use std::cmp::Ordering;
use std::collections::BTreeSet;

mod card_id_filter;
mod icons;
mod review_command;
mod review_plots;
mod table_fields_state;
mod table_filter_state;
mod table_sort_state;
mod transactions_review;

use card_id_filter::{
    CardIdFilter, card_id_matches_filter, normalize_card_id_query, parse_card_id_filter,
};
use icons::paint_sort_icon;
use table_fields_state::TableFieldsState;
use table_filter_state::TableFilterState;
use table_sort_state::TableSortState;

pub(crate) use transactions_review::show_flagged_transactions_review;

#[derive(Debug, Clone)]
pub struct ReviewActionLogEntry {
    pub label: String,
    pub changed_count: usize,
    pub summary: String,
    pub timestamp: String,
}

pub fn show_review_action_history(ui: &mut egui::Ui, history: &[ReviewActionLogEntry]) {
    if history.is_empty() {
        ui.label("No review actions yet.");
        return;
    }

    ui.label(format!("{} actions", history.len()));
    ui.add_space(6.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("review_action_history_table")
            .striped(true)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Time");
                ui.strong("Action");
                ui.strong("Tx");
                ui.strong("Details");
                ui.end_row();

                for entry in history.iter().rev() {
                    let action_color = action_color_for_label(&entry.label);
                    ui.label(
                        egui::RichText::new(&entry.timestamp).color(ui.visuals().weak_text_color()),
                    );
                    ui.label(
                        egui::RichText::new(&entry.label)
                            .color(action_color)
                            .strong(),
                    );
                    ui.label(entry.changed_count.to_string());
                    ui.label(&entry.summary);
                    ui.end_row();
                }
            });
    });
}

fn action_color_for_label(label: &str) -> egui::Color32 {
    let lower = label.to_ascii_lowercase();
    if lower.contains("fraud") {
        egui::Color32::from_rgb(225, 90, 90)
    } else if lower.contains("approve") {
        egui::Color32::from_rgb(120, 180, 120)
    } else if lower.contains("undo") {
        egui::Color32::from_rgb(170, 170, 220)
    } else if lower.contains("redo") {
        egui::Color32::from_rgb(140, 190, 235)
    } else {
        egui::Color32::from_rgb(250, 235, 175)
    }
}

const FIELD_COUNT: usize = 13;

const FIELD_TITLES: [&str; FIELD_COUNT] = [
    "Transaction ID",
    "Timestamp",
    "Card ID",
    "Amount",
    "Merchant",
    "Merchant Category",
    "Channel",
    "Cardholder Country",
    "Merchant Country",
    "Device ID",
    "IP Address",
    "Fraud",
    "Fraud Signals",
];

struct TransactionsTable<'a> {
    rows: &'a [Transaction],
    row_indices: &'a [usize],
    visible_fields: &'a [usize],
    sort_state: TableSortState,
}

impl<'a> egui_table::TableDelegate for TransactionsTable<'a> {
    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::HeaderCellInfo) {
        let Some(&field_idx) = self.visible_fields.get(cell.col_range.start) else {
            return;
        };
        let title = FIELD_TITLES[field_idx];
        let is_active_sort = self.sort_state.field_idx == Some(field_idx);

        ui.horizontal(|ui| {
            ui.add_space(6.0);
            let button_label = if is_active_sort {
                format!("{title}    ")
            } else {
                title.to_owned()
            };
            let button_response = ui.button(button_label);

            if button_response.clicked() {
                if is_active_sort {
                    self.sort_state.descending = !self.sort_state.descending;
                } else {
                    self.sort_state.field_idx = Some(field_idx);
                    self.sort_state.descending = false;
                }
            }

            if is_active_sort {
                let icon_rect = egui::Rect::from_center_size(
                    egui::pos2(
                        button_response.rect.right() - 8.0,
                        button_response.rect.center().y,
                    ),
                    egui::vec2(8.0, 8.0),
                );
                paint_sort_icon(
                    ui.painter(),
                    icon_rect,
                    self.sort_state.descending,
                    ui.visuals().widgets.active.fg_stroke.color,
                );
            }
            ui.add_space(6.0);
        });
    }

    fn row_ui(&mut self, ui: &mut egui::Ui, row_nr: u64) {
        if row_nr % 2 == 1 {
            let mut stripe = ui.visuals().faint_bg_color;
            stripe = stripe.linear_multiply(0.7);
            ui.painter().rect_filled(ui.max_rect(), 0.0, stripe);
        }
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::CellInfo) {
        let Some(&row_idx) = self.row_indices.get(cell.row_nr as usize) else {
            return;
        };
        let Some(row) = self.rows.get(row_idx) else {
            return;
        };
        let Some(&field_idx) = self.visible_fields.get(cell.col_nr) else {
            return;
        };

        if field_idx == 11 {
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                if row.fraud_factors.is_empty() {
                    ui.label("-");
                } else if matches!(row.human_review_status, HumanReviewStatus::FalsePositive) {
                    ui.colored_label(
                        egui::Color32::from_rgb(120, 180, 120),
                        format!("reviewed clean ({:.2})", row.fraud_score()),
                    );
                } else if matches!(row.human_review_status, HumanReviewStatus::TruePositive) {
                    ui.colored_label(
                        egui::Color32::from_rgb(225, 90, 90),
                        format!("reviewed fraud ({:.2})", row.fraud_score()),
                    );
                } else if row.likely_fraud() {
                    ui.colored_label(
                        egui::Color32::from_rgb(225, 90, 90),
                        format!("likely ({:.2})", row.fraud_score()),
                    );
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(210, 180, 80),
                        format!("signal ({:.2})", row.fraud_score()),
                    );
                }
                ui.add_space(6.0);
            });
            return;
        }

        if field_idx == 12 {
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                if row.fraud_factors.is_empty() {
                    ui.label("-");
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for factor in &row.fraud_factors {
                            render_reason_chip_overview(ui, factor);
                        }
                    });
                }
                ui.add_space(6.0);
            });
            return;
        }

        let text = match field_idx {
            0 => format!("{}", row.transaction_id.0),
            1 => row.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            2 => format!("{}", row.card_id.0),
            3 => format!("{:.2} $", row.amount),
            4 => row.merchant_name.clone(),
            5 => format!("{:?}", row.merchant_category),
            6 => format!("{:?}", row.channel),
            7 => country_label(row.cardholder_country),
            8 => country_label(row.merchant_country),
            9 => row.device_id.clone().unwrap_or_else(|| "-".to_owned()),
            10 => row
                .ip_address
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            _ => String::new(),
        };
        if field_idx == 3 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                ui.label(text);
                ui.add_space(6.0);
            });
        } else {
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                ui.label(text);
                ui.add_space(6.0);
            });
        }
    }
}

pub fn show_transactions_table(ui: &mut egui::Ui, rows: &[Transaction]) {
    let fields_state_id = ui.make_persistent_id("transactions_table_visible_fields");
    let sort_state_id = ui.make_persistent_id("transactions_table_sort");
    let filter_state_id = ui.make_persistent_id("transactions_table_filter");

    let mut fields_state = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<TableFieldsState>(fields_state_id))
        .unwrap_or_default();
    let mut sort_state = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<TableSortState>(sort_state_id))
        .unwrap_or_default();
    let mut filter_state = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<TableFilterState>(filter_state_id))
        .unwrap_or_default();

    let available_card_ids: Vec<u64> = rows
        .iter()
        .map(|row| row.card_id.0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    ui.horizontal(|ui| {
        ui.label("Filter card ID:");

        let text_edit_response = ui.add(
            egui::TextEdit::singleline(&mut filter_state.card_id_query)
                .hint_text("e.g. 1234")
                .desired_width(160.0),
        );

        let normalized_query = normalize_card_id_query(&filter_state.card_id_query);
        let parsed_filter = parse_card_id_filter(&filter_state.card_id_query);
        let suggestions: Vec<u64> = available_card_ids
            .iter()
            .copied()
            .filter(|card_id| {
                normalized_query.is_empty() || card_id.to_string().contains(&normalized_query)
            })
            .collect();

        if text_edit_response.has_focus() && !suggestions.is_empty() {
            filter_state.autocomplete_open = true;
        }
        if text_edit_response.changed() {
            filter_state.autocomplete_open = !suggestions.is_empty();
        }
        if suggestions.is_empty() {
            filter_state.autocomplete_open = false;
        }

        let row_height = text_edit_response.rect.height();
        let input_width = text_edit_response.rect.width();

        let mut popup_open = filter_state.autocomplete_open;
        let mut close_popup_after_selection = false;

        egui::Popup::from_response(&text_edit_response)
            .id(ui.make_persistent_id("card_id_filter_autocomplete_popup"))
            .open_bool(&mut popup_open)
            .frame(
                egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::same(0))
                    .outer_margin(egui::Margin::same(0)),
            )
            .width(input_width)
            .show(|ui| {
                ui.set_width(input_width);
                ui.set_min_width(input_width);
                ui.set_max_width(input_width);

                let clear_selected = ui
                    .add_sized(
                        [input_width, row_height],
                        egui::Button::new("Any card")
                            .selected(matches!(parsed_filter, CardIdFilter::Any)),
                    )
                    .clicked();
                if clear_selected
                {
                    filter_state.card_id_query.clear();
                    close_popup_after_selection = true;
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(180.0)
                    .show(ui, |ui| {
                        ui.set_width(input_width);
                        ui.set_min_width(input_width);
                        ui.set_max_width(input_width);

                        for &card_id in &suggestions {
                            let label = format!("Card {card_id}");
                            let is_selected =
                                matches!(parsed_filter, CardIdFilter::Exact(selected) if selected == card_id);

                            let selected = ui
                                .add_sized(
                                    [input_width, row_height],
                                    egui::Button::new(&label).selected(is_selected),
                                )
                                .clicked();
                            if selected {
                                filter_state.card_id_query = card_id.to_string();
                                close_popup_after_selection = true;
                            }
                        }
                    });
            });

        if close_popup_after_selection {
            popup_open = false;
        }
        filter_state.autocomplete_open = popup_open;

        if !filter_state.card_id_query.trim().is_empty() {
            let clear_clicked = ui
                .add_sized(
                    [60.0, text_edit_response.rect.height()],
                    egui::Button::new("Clear"),
                )
                .clicked();
            if clear_clicked {
                filter_state.card_id_query.clear();
                filter_state.autocomplete_open = false;
            }
        }

        ui.add_space(10.0);
        ui.checkbox(
            &mut filter_state.reviewed_or_marked_only,
            "Reviewed or system-marked only",
        );
    });

    egui::CollapsingHeader::new("Columns")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (idx, title) in FIELD_TITLES.iter().enumerate() {
                    ui.checkbox(&mut fields_state.visible[idx], *title);
                }
            });
        });

    if !fields_state.visible.iter().any(|v| *v) {
        fields_state.visible[0] = true;
    }

    ui.ctx()
        .data_mut(|d| d.insert_persisted(fields_state_id, fields_state.clone()));
    ui.ctx()
        .data_mut(|d| d.insert_persisted(filter_state_id, filter_state.clone()));

    let visible_fields: Vec<usize> = fields_state
        .visible
        .iter()
        .enumerate()
        .filter_map(|(idx, visible)| visible.then_some(idx))
        .collect();

    let parsed_filter = parse_card_id_filter(&filter_state.card_id_query);
    let reviewed_or_marked_only = filter_state.reviewed_or_marked_only;
    let mut row_indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            let passes_card_filter = card_id_matches_filter(row.card_id.0, parsed_filter);
            let passes_review_filter = !reviewed_or_marked_only
                || matches!(
                    row.human_review_status,
                    HumanReviewStatus::FalsePositive | HumanReviewStatus::TruePositive
                )
                || !row.fraud_factors.is_empty();
            (passes_card_filter && passes_review_filter).then_some(idx)
        })
        .collect();

    if let Some(field_idx) = sort_state.field_idx {
        row_indices.sort_by(|&left, &right| {
            compare_rows(&rows[left], &rows[right], field_idx, sort_state.descending)
        });
    }

    if !matches!(parsed_filter, CardIdFilter::Any) || reviewed_or_marked_only {
        ui.label(format!(
            "Showing {} of {} transactions.",
            row_indices.len(),
            rows.len()
        ));
    }

    let columns: Vec<egui_table::Column> = visible_fields
        .iter()
        .map(|&_| egui_table::Column::new(300.0))
        .collect();

    let mut table_delegate = TransactionsTable {
        rows,
        row_indices: &row_indices,
        visible_fields: &visible_fields,
        sort_state: sort_state.clone(),
    };
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 10.0;

    egui_table::Table::new()
        .id_salt("transactions_table")
        .columns(columns)
        .auto_size_mode(egui_table::AutoSizeMode::OnParentResize)
        .headers([egui_table::HeaderRow::new(row_height)])
        .num_rows(row_indices.len() as u64)
        .show(ui, &mut table_delegate);

    sort_state = table_delegate.sort_state;
    ui.ctx()
        .data_mut(|d| d.insert_persisted(sort_state_id, sort_state));
}

fn compare_rows(
    left: &Transaction,
    right: &Transaction,
    field_idx: usize,
    descending: bool,
) -> Ordering {
    let mut ordering = match field_idx {
        0 => left.transaction_id.0.cmp(&right.transaction_id.0),
        1 => left.timestamp.cmp(&right.timestamp),
        2 => left.card_id.0.cmp(&right.card_id.0),
        3 => left.amount.total_cmp(&right.amount),
        4 => left.merchant_name.cmp(&right.merchant_name),
        5 => format!("{:?}", left.merchant_category).cmp(&format!("{:?}", right.merchant_category)),
        6 => format!("{:?}", left.channel).cmp(&format!("{:?}", right.channel)),
        7 => {
            format!("{:?}", left.cardholder_country).cmp(&format!("{:?}", right.cardholder_country))
        }
        8 => format!("{:?}", left.merchant_country).cmp(&format!("{:?}", right.merchant_country)),
        9 => left.device_id.cmp(&right.device_id),
        10 => left.ip_address.cmp(&right.ip_address),
        11 => left
            .likely_fraud_for_export()
            .cmp(&right.likely_fraud_for_export())
            .then_with(|| left.fraud_score().total_cmp(&right.fraud_score())),
        12 => left
            .fraud_factors
            .iter()
            .map(short_reason)
            .collect::<Vec<_>>()
            .join(" | ")
            .cmp(
                &right
                    .fraud_factors
                    .iter()
                    .map(short_reason)
                    .collect::<Vec<_>>()
                    .join(" | "),
            ),
        _ => Ordering::Equal,
    };

    if descending {
        ordering = ordering.reverse();
    }

    if ordering == Ordering::Equal {
        left.transaction_id.0.cmp(&right.transaction_id.0)
    } else {
        ordering
    }
}

fn render_reason_chip(ui: &mut egui::Ui, factor: &FraudFactor) {
    render_reason_chip_inner(ui, factor, false);
}

fn render_reason_chip_overview(ui: &mut egui::Ui, factor: &FraudFactor) {
    render_reason_chip_inner(ui, factor, true);
}

fn render_reason_chip_inner(ui: &mut egui::Ui, factor: &FraudFactor, compact: bool) {
    let (fill, text_color) = if factor.weight() >= 0.9 {
        (
            egui::Color32::from_rgb(80, 28, 28),
            egui::Color32::from_rgb(255, 210, 210),
        )
    } else {
        (
            egui::Color32::from_rgb(74, 64, 28),
            egui::Color32::from_rgb(250, 235, 175),
        )
    };
    let text = if compact {
        short_reason(factor)
    } else {
        factor.reason()
    };

    egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(4, 2))
        .show(ui, |ui| {
            if compact {
                ui.label(egui::RichText::new(text).color(text_color));
            } else {
                ui.label(egui::RichText::new(text).color(text_color));
            }
        })
        .response;
}

fn short_reason(factor: &FraudFactor) -> String {
    match factor {
        FraudFactor::ForeignCountryTrip {
            country,
            primary_country,
            likely_vacation,
            ..
        } => {
            if *likely_vacation {
                format!(
                    "travel {}>{}",
                    primary_country.0.alpha2(),
                    country.0.alpha2()
                )
            } else {
                format!(
                    "country jump {}>{}",
                    primary_country.0.alpha2(),
                    country.0.alpha2()
                )
            }
        }
        FraudFactor::CardTestingBurst {
            transaction_count,
            max_amount,
            max_gap,
            ..
        } => {
            format!(
                "card testing burst: {transaction_count} tx <= {:.2}, max gap {}s",
                max_amount,
                max_gap.num_seconds()
            )
        }
        FraudFactor::InactiveCardTestingBurst {
            transaction_count,
            max_amount,
            max_gap,
            ..
        } => {
            format!(
                "card testing burst (resolved): {transaction_count} tx <= {:.2}, max gap {}s",
                max_amount,
                max_gap.num_seconds()
            )
        }
    }
}

fn country_label(country: model::data::country::Country) -> String {
    if let Some(common_name) = country.0.unofficial_names().first() {
        return common_name.to_string();
    }

    country.0.iso_short_name().to_string()
}
