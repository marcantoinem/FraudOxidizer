use model::data::transactions::Transactions;

#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Default)]
pub struct CsvState {
    pub transactions: Option<Transactions>,
    pub picked_name: Option<String>,
    pub parse_error: Option<String>,
    pub last_loaded_csv_content: Option<String>,
    loaded_valid_csv: bool,
    #[cfg(target_arch = "wasm32")]
    pending_csv_tx: Sender<(String, Vec<u8>)>,
    #[cfg(target_arch = "wasm32")]
    pending_csv_rx: Receiver<(String, Vec<u8>)>,
}

impl CsvState {
    pub fn load_csv_content(&mut self, name: String, content: String) {
        self.picked_name = Some(name);
        match Transactions::parse_csv_content(&content) {
            Ok(mut transactions) => {
                transactions.apply_fraud_factors();
                self.transactions = Some(transactions);
                self.parse_error = None;
                self.last_loaded_csv_content = Some(content);
                self.loaded_valid_csv = true;
            }
            Err(e) => {
                self.transactions = None;
                self.parse_error = Some(e.to_string());
                self.last_loaded_csv_content = None;
                self.loaded_valid_csv = false;
            }
        }
    }

    pub fn load_csv_from_bytes(&mut self, name: String, bytes: &[u8]) {
        match std::str::from_utf8(bytes) {
            Ok(content) => self.load_csv_content(name, content.to_owned()),
            Err(e) => {
                self.picked_name = Some(name);
                self.transactions = None;
                self.parse_error = Some(e.to_string());
                self.last_loaded_csv_content = None;
                self.loaded_valid_csv = false;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_csv_from_path(&mut self, path: &std::path::Path) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        match std::fs::read(path) {
            Ok(bytes) => self.load_csv_from_bytes(name, &bytes),
            Err(e) => {
                self.picked_name = Some(path.display().to_string());
                self.transactions = None;
                self.parse_error = Some(e.to_string());
                self.last_loaded_csv_content = None;
                self.loaded_valid_csv = false;
            }
        }
    }

    pub fn load_csv_from_dropped_file(&mut self, file: egui::DroppedFile) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = file.path {
            self.load_csv_from_path(&path);
            return;
        }

        if let Some(bytes) = file.bytes {
            let name = if file.name.is_empty() {
                "dropped_file.csv".to_owned()
            } else {
                file.name
            };
            self.load_csv_from_bytes(name, &bytes);
        }
    }

    pub fn take_loaded_valid_csv_event(&mut self) -> bool {
        let loaded = self.loaded_valid_csv;
        self.loaded_valid_csv = false;
        loaded
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_csv_dialog(&mut self, _ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .pick_file()
        {
            self.load_csv_from_path(&path);
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open_csv_dialog(&mut self, ctx: &egui::Context) {
        let tx = self.pending_csv_tx.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("CSV", &["csv"])
                .pick_file()
                .await
            {
                let name = file.file_name();
                let bytes = file.read().await;
                let _ = tx.send((name, bytes));
                ctx.request_repaint();
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub fn poll_web_dialog_result(&mut self) {
        while let Ok((name, bytes)) = self.pending_csv_rx.try_recv() {
            self.load_csv_from_bytes(name, &bytes);
        }
    }
}
