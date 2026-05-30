use model::data::transactions::Transactions;

#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::{self, Receiver, Sender};

pub struct CsvState {
    pub transactions: Option<Transactions>,
    pub picked_name: Option<String>,
    pub parse_error: Option<String>,
    #[cfg(target_arch = "wasm32")]
    pending_csv_tx: Sender<(String, Vec<u8>)>,
    #[cfg(target_arch = "wasm32")]
    pending_csv_rx: Receiver<(String, Vec<u8>)>,
}

impl Default for CsvState {
    fn default() -> Self {
        #[cfg(target_arch = "wasm32")]
        let (pending_csv_tx, pending_csv_rx) = mpsc::channel();

        Self {
            transactions: None,
            picked_name: None,
            parse_error: None,
            #[cfg(target_arch = "wasm32")]
            pending_csv_tx,
            #[cfg(target_arch = "wasm32")]
            pending_csv_rx,
        }
    }
}

impl CsvState {
    pub fn load_csv_from_bytes(&mut self, name: String, bytes: &[u8]) {
        let loaded = parse_csv_bytes(name, bytes);
        self.picked_name = loaded.picked_name;
        self.transactions = loaded.transactions;
        self.parse_error = loaded.parse_error;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_csv_from_path(&mut self, path: &std::path::Path) {
        let loaded = parse_csv_path(path);
        self.picked_name = loaded.picked_name;
        self.transactions = loaded.transactions;
        self.parse_error = loaded.parse_error;
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

pub fn parse_csv_bytes(name: String, bytes: &[u8]) -> CsvLoadResult {
    let parse_result = std::str::from_utf8(bytes)
        .map_err(|e| e.to_string())
        .and_then(|content| Transactions::parse_csv_content(content).map_err(|e| e.to_string()));

    match parse_result {
        Ok(transactions) => CsvLoadResult {
            picked_name: Some(name),
            transactions: Some(transactions),
            parse_error: None,
        },
        Err(parse_error) => CsvLoadResult {
            picked_name: Some(name),
            transactions: None,
            parse_error: Some(parse_error),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_csv_path(path: &std::path::Path) -> CsvLoadResult {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    match std::fs::read(path) {
        Ok(bytes) => parse_csv_bytes(name, &bytes),
        Err(e) => CsvLoadResult {
            picked_name: Some(path.display().to_string()),
            transactions: None,
            parse_error: Some(e.to_string()),
        },
    }
}

pub struct CsvLoadResult {
    pub picked_name: Option<String>,
    pub transactions: Option<Transactions>,
    pub parse_error: Option<String>,
}
