use model::data::transaction::Transaction;
use std::cmp::Ordering;
use std::collections::BTreeSet;

const FIELD_COUNT: usize = 11;

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
];

const FIELD_WIDTHS: [f32; FIELD_COUNT] = [
    140.0, 170.0, 110.0, 90.0, 220.0, 160.0, 100.0, 150.0, 150.0, 140.0, 170.0,
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
        .map(|&field_idx| egui_table::Column::new(FIELD_WIDTHS[field_idx]))
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
