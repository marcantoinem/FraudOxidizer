use crate::csv_loader::CsvState;
use crate::transactions_table;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
struct PersistedAppState {
    picked_name: Option<String>,
    last_loaded_csv_content: Option<String>,
}

pub struct TemplateApp {
    csv: CsvState,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            csv: CsvState::default(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();

        if let Some(storage) = cc.storage
            && let Some(persisted) =
                eframe::get_value::<PersistedAppState>(storage, eframe::APP_KEY)
            && let Some(content) = persisted.last_loaded_csv_content
        {
            let name = persisted
                .picked_name
                .unwrap_or_else(|| "restored.csv".to_owned());
            app.csv.load_csv_content(name, content);
        }

        app
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let persisted = PersistedAppState {
            picked_name: self.csv.picked_name.clone(),
            last_loaded_csv_content: self.csv.last_loaded_csv_content.clone(),
        };
        eframe::set_value(storage, eframe::APP_KEY, &persisted);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ui.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Fraud Detector");

            ui.label("Drag-and-drop a CSV file onto the window, or use the button below.");

            if ui.button("Open CSV file…").clicked() {
                self.csv.open_csv_dialog(ui.ctx());
            }

            if let Some(picked_path) = &self.csv.picked_name {
                ui.horizontal(|ui| {
                    ui.label("Loaded file:");
                    ui.monospace(picked_path);
                });
            }

            if let Some(err) = &self.csv.parse_error {
                let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(180, 40, 40));
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(58, 20, 20))
                    .stroke(stroke)
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 200, 200),
                                "CSV Parse Error",
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(err)
                                    .color(egui::Color32::from_rgb(255, 230, 230)),
                            );
                        });
                    });
            }

            if let Some(transactions) = &self.csv.transactions {
                ui.separator();
                ui.label(format!("Loaded {} transactions.", transactions.items.len()));
                ui.separator();
                transactions_table::show_transactions_table(ui, &transactions.items);
            }
        });

        preview_files_being_dropped(ui.ctx());

        // Poll for async file dialog result (web):
        #[cfg(target_arch = "wasm32")]
        self.csv.poll_web_dialog_result();

        // Collect dropped files:
        let dropped: Vec<egui::DroppedFile> = ui.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            self.csv.load_csv_from_dropped_file(file);
            break;
        }
    }
}

fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Drop CSV file:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if file.mime.is_empty() {
                    text += "\n???";
                } else {
                    write!(text, "\n{}", file.mime).ok();
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let content_rect = ctx.content_rect();
        painter.rect_filled(content_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            content_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.global_style()),
            Color32::WHITE,
        );
    }
}
