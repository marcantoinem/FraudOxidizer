use model::data::human_review_status::HumanReviewStatus;
use model::data::transaction::Transaction;
use model::process::card_statistics::FraudFactor;
use std::cmp::Ordering;
use std::collections::BTreeSet;

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

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct TableFieldsState {
    visible: [bool; FIELD_COUNT],
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Default)]
struct TableSortState {
    field_idx: Option<usize>,
    descending: bool,
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Default)]
struct TableFilterState {
    card_id_query: String,
    autocomplete_open: bool,
}

impl Default for TableFieldsState {
    fn default() -> Self {
        Self {
            visible: [true; FIELD_COUNT],
        }
    }
}

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
                            render_reason_chip(ui, factor);
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
    let mut row_indices: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            card_id_matches_filter(row.card_id.0, parsed_filter).then_some(idx)
        })
        .collect();

    if let Some(field_idx) = sort_state.field_idx {
        row_indices.sort_by(|&left, &right| {
            compare_rows(&rows[left], &rows[right], field_idx, sort_state.descending)
        });
    }

    if !matches!(parsed_filter, CardIdFilter::Any) {
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

    let response = egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(4, 1))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(short_reason(factor)).color(text_color));
        })
        .response;

    response.on_hover_text(factor.reason());
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
    }
}

pub fn show_flagged_transactions_review(ui: &mut egui::Ui, rows: &mut [Transaction]) {
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
    let mut cursor = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<usize>(cursor_id))
        .unwrap_or(0);

    let item_count = flagged_indices.len();
    if cursor >= item_count {
        cursor = item_count - 1;
    }

    let previous_shortcut =
        egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowLeft);
    let next_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::ArrowRight);
    let approve_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::A);
    let fraud_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F);
    let reset_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::R);

    let previous_triggered = ui
        .ctx()
        .input_mut(|i| i.consume_shortcut(&previous_shortcut));
    let next_triggered = ui.ctx().input_mut(|i| i.consume_shortcut(&next_shortcut));
    let approve_triggered = ui
        .ctx()
        .input_mut(|i| i.consume_shortcut(&approve_shortcut));
    let fraud_triggered = ui.ctx().input_mut(|i| i.consume_shortcut(&fraud_shortcut));
    let reset_triggered = ui.ctx().input_mut(|i| i.consume_shortcut(&reset_shortcut));

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

    let row_index = flagged_indices[cursor];
    let row = &mut rows[row_index];

    ui.horizontal(|ui| {
        let previous_clicked = ui.button("Previous").clicked();
        if previous_clicked {
            cursor = if cursor == 0 {
                item_count - 1
            } else {
                cursor - 1
            };
        }
        ui.strong(format!("{} / {}", cursor + 1, item_count));
        let next_clicked = ui.button("Next").clicked();
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
                    render_reason_chip(ui, factor);
                }
            });

            ui.add_space(10.0);
            let mut advance_after_action = false;
            ui.horizontal(|ui| {
                let approve_clicked = ui.button("Approve").clicked();
                if approve_clicked || approve_triggered {
                    row.human_review_status = HumanReviewStatus::FalsePositive;
                    advance_after_action = true;
                }
                let fraud_clicked = ui.button("Mark fraud").clicked();
                if fraud_clicked || fraud_triggered {
                    row.human_review_status = HumanReviewStatus::TruePositive;
                    advance_after_action = true;
                }
                let reset_clicked = ui.button("Reset").clicked();
                if reset_clicked || reset_triggered {
                    row.human_review_status = HumanReviewStatus::NeedCheck;
                }
            });

            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                shortcut_legend_item(ui, &reset_shortcut, "reset");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &fraud_shortcut, "mark fraud");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &approve_shortcut, "approve");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &next_shortcut, "next");
                ui.add_space(10.0);
                shortcut_legend_item(ui, &previous_shortcut, "previous");
            });

            if advance_after_action && item_count > 1 {
                cursor = (cursor + 1) % item_count;
            }
        });

    ui.ctx().data_mut(|d| d.insert_persisted(cursor_id, cursor));
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

fn paint_sort_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    descending: bool,
    color: egui::Color32,
) {
    let center = rect.center();
    let half_w = 3.5;
    let half_h = 2.5;

    let points = if descending {
        vec![
            egui::pos2(center.x - half_w, center.y - half_h),
            egui::pos2(center.x + half_w, center.y - half_h),
            egui::pos2(center.x, center.y + half_h),
        ]
    } else {
        vec![
            egui::pos2(center.x - half_w, center.y + half_h),
            egui::pos2(center.x + half_w, center.y + half_h),
            egui::pos2(center.x, center.y - half_h),
        ]
    };

    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

fn country_label(country: model::data::country::Country) -> String {
    if let Some(common_name) = country.0.unofficial_names().first() {
        return normalize_country_name(common_name);
    }

    normalize_country_name(country.0.iso_short_name())
}

fn normalize_country_name(name: &str) -> String {
    name.trim()
        .trim_end_matches(" (the)")
        .trim_end_matches(", The")
        .to_owned()
}

#[derive(Clone, Copy)]
enum CardIdFilter {
    Any,
    Exact(u64),
    Invalid,
}

fn parse_card_id_filter(query: &str) -> CardIdFilter {
    let normalized = normalize_card_id_query(query);
    if normalized.is_empty() {
        return CardIdFilter::Any;
    }

    match normalized.parse::<u64>() {
        Ok(card_id) => CardIdFilter::Exact(card_id),
        Err(_) => CardIdFilter::Invalid,
    }
}

fn normalize_card_id_query(query: &str) -> String {
    let normalized = query.trim().to_ascii_lowercase();
    normalized
        .strip_prefix("card_")
        .unwrap_or(&normalized)
        .to_owned()
}

fn card_id_matches_filter(card_id: u64, filter: CardIdFilter) -> bool {
    match filter {
        CardIdFilter::Any => true,
        CardIdFilter::Exact(selected) => card_id == selected,
        CardIdFilter::Invalid => false,
    }
}
