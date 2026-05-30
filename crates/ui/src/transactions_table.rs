use model::data::transaction::Transaction;

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

impl Default for TableFieldsState {
    fn default() -> Self {
        Self {
            visible: [true; FIELD_COUNT],
        }
    }
}

struct TransactionsTable<'a> {
    rows: &'a [Transaction],
    visible_fields: &'a [usize],
}

impl<'a> egui_table::TableDelegate for TransactionsTable<'a> {
    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::HeaderCellInfo) {
        let Some(&field_idx) = self.visible_fields.get(cell.col_range.start) else {
            return;
        };
        let title = FIELD_TITLES[field_idx];

        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.strong(title);
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
        let Some(row) = self.rows.get(cell.row_nr as usize) else {
            return;
        };
        let Some(&field_idx) = self.visible_fields.get(cell.col_nr) else {
            return;
        };

        let text = match field_idx {
            0 => format!("{}", row.transaction_id.0),
            1 => row.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            2 => format!("{}", row.card_id.0),
            3 => format!("{:.2}", row.amount),
            4 => row.merchant_name.clone(),
            5 => format!("{:?}", row.merchant_category),
            6 => format!("{:?}", row.channel),
            7 => format!("{:?}", row.cardholder_country),
            8 => format!("{:?}", row.merchant_country),
            9 => row.device_id.clone().unwrap_or_else(|| "-".to_owned()),
            10 => row
                .ip_address
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "-".to_owned()),
            _ => String::new(),
        };
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(text);
            ui.add_space(6.0);
        });
    }
}

pub fn show_transactions_table(ui: &mut egui::Ui, rows: &[Transaction]) {
    let state_id = ui.make_persistent_id("transactions_table_visible_fields");
    let mut fields_state = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<TableFieldsState>(state_id))
        .unwrap_or_default();

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
        .data_mut(|d| d.insert_persisted(state_id, fields_state.clone()));

    let visible_fields: Vec<usize> = fields_state
        .visible
        .iter()
        .enumerate()
        .filter_map(|(idx, visible)| visible.then_some(idx))
        .collect();

    let columns: Vec<egui_table::Column> = visible_fields
        .iter()
        .map(|&field_idx| egui_table::Column::new(FIELD_WIDTHS[field_idx]))
        .collect();

    let mut table_delegate = TransactionsTable {
        rows,
        visible_fields: &visible_fields,
    };
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 10.0;

    egui_table::Table::new()
        .id_salt("transactions_table")
        .columns(columns)
        .headers([egui_table::HeaderRow::new(row_height)])
        .num_rows(rows.len() as u64)
        .show(ui, &mut table_delegate);
}
