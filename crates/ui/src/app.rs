use crate::csv_loader::CsvState;
use crate::state::Progression;
use crate::transactions_table;
use model::process::card_statistics::HUMAN_REVIEW_SCORE_THRESHOLD_DEFAULT;

use egui::{CentralPanel, Panel};
#[cfg(target_arch = "wasm32")]
use web_sys::wasm_bindgen::JsCast as _;

fn default_human_review_score_threshold() -> f32 {
    HUMAN_REVIEW_SCORE_THRESHOLD_DEFAULT
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
struct PersistedAppState {
    picked_name: Option<String>,
    last_loaded_csv_content: Option<String>,
    current_step: Progression,
    #[serde(default = "default_human_review_score_threshold")]
    human_review_score_threshold: f32,
}

pub struct FraudOxidizerApp {
    csv: CsvState,
    current_step: Progression,
    human_review_score_threshold: f32,
    review_action_history: Vec<transactions_table::ReviewActionLogEntry>,
}

impl Default for FraudOxidizerApp {
    fn default() -> Self {
        Self {
            csv: CsvState::default(),
            current_step: Progression::default(),
            human_review_score_threshold: HUMAN_REVIEW_SCORE_THRESHOLD_DEFAULT,
            review_action_history: Vec::new(),
        }
    }
}

impl FraudOxidizerApp {
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
            let _ = app.csv.take_loaded_valid_csv_event();
            app.current_step = persisted.current_step;
            app.human_review_score_threshold = persisted.human_review_score_threshold;
        }

        app
    }
}

impl eframe::App for FraudOxidizerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let persisted = PersistedAppState {
            picked_name: self.csv.picked_name.clone(),
            last_loaded_csv_content: self.csv.last_loaded_csv_content.clone(),
            current_step: self.current_step,
            human_review_score_threshold: self.human_review_score_threshold,
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
            ui.heading("FraudOxidizer");
            ui.add_space(6.0);
            show_stepper(ui, &mut self.current_step, self.csv.transactions.is_some());
            ui.add_space(12.0);

            if self.csv.take_loaded_valid_csv_event() {
                self.review_action_history.clear();
                self.current_step = self.current_step.next();
            }

            match self.current_step {
                Progression::ImportCsv => self.show_import_step(ui),
                Progression::CheckProbableFraud => self.show_review_step(ui),
                Progression::ExportCsvView => self.show_overview_step(ui),
            }
        });

        preview_files_being_dropped(ui.ctx());

        // Poll for async file dialog result (web):
        #[cfg(target_arch = "wasm32")]
        self.csv.poll_web_dialog_result();

        // Collect dropped files:
        let dropped: Vec<egui::DroppedFile> = ui.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped.into_iter().next() {
            self.csv.load_csv_from_dropped_file(file);
        }
    }
}

impl FraudOxidizerApp {
    fn show_import_step(&mut self, ui: &mut egui::Ui) {
        ui.label("Step 1: import a CSV file. Once a valid CSV is received, the app advances to review automatically.");
        ui.add_space(8.0);

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
                        ui.colored_label(egui::Color32::from_rgb(255, 200, 200), "CSV Parse Error");
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(err).color(egui::Color32::from_rgb(255, 230, 230)),
                        );
                    });
                });
        }

        if let Some(transactions) = &self.csv.transactions {
            ui.add_space(8.0);
            ui.label(format!("Loaded {} transactions.", transactions.items.len()));
        }

        ui.add_space(12.0);
        if ui
            .add_enabled(
                self.csv.transactions.is_some(),
                egui::Button::new("Continue to human review"),
            )
            .clicked()
        {
            self.current_step = self.current_step.next();
        }
    }

    fn show_review_step(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Step 2: review flagged transactions");
            ui.label("Shortcut labels are shown directly on the carousel controls.");
        });
        ui.add_space(8.0);

        if let Some(transactions) = &mut self.csv.transactions {
            Panel::right(ui.id().with("review_activity_side"))
                .resizable(true)
                .show_inside(ui, |ui| {
                    ui.heading("Review Activity");
                    ui.add_space(6.0);
                    transactions_table::show_review_action_history(ui, &self.review_action_history);
                });
            CentralPanel::default_margins().show_inside(ui, |ui| {
                transactions_table::show_flagged_transactions_review(
                    ui,
                    &mut transactions.items,
                    &mut self.human_review_score_threshold,
                    &mut self.review_action_history,
                );
            });
        } else {
            ui.label("Load a CSV first.");
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                self.current_step = self.current_step.previous();
            }
            if ui
                .add_enabled(
                    self.csv.transactions.is_some(),
                    egui::Button::new("Go to overview"),
                )
                .clicked()
            {
                self.current_step = self.current_step.next();
            }
        });
    }

    fn show_overview_step(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Step 3: overview and export the reviewed CSV.");
            ui.label("The export uses the human review decision when one exists, otherwise it falls back to the fraud threshold.");
        });
        ui.add_space(8.0);

        if let Some(transactions) = &self.csv.transactions {
            ui.horizontal(|ui| {
                if ui.button("Export reviewed CSV").clicked() {
                    export_reviewed_csv(ui.ctx(), transactions, self.csv.picked_name.as_deref());
                }
                if ui.button("Back to review").clicked() {
                    self.current_step = self.current_step.previous();
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label(format!("Loaded {} transactions.", transactions.items.len()));
            ui.separator();
            transactions_table::show_transactions_table(ui, &transactions.items);
        } else {
            ui.label("Load a CSV first.");
        }
    }
}

fn show_stepper(ui: &mut egui::Ui, current_step: &mut Progression, has_csv: bool) {
    let steps = [
        Progression::ImportCsv,
        Progression::CheckProbableFraud,
        Progression::ExportCsvView,
    ];

    ui.horizontal(|ui| {
        for (index, step) in steps.iter().copied().enumerate() {
            let enabled = has_csv || matches!(step, Progression::ImportCsv);
            let selected = *current_step == step;
            let completed = current_step.is_after(step);
            let (fill, stroke, text_color) = if selected {
                (
                    ui.visuals().selection.bg_fill,
                    egui::Stroke::new(1.5, ui.visuals().selection.stroke.color),
                    ui.visuals().selection.stroke.color,
                )
            } else if completed {
                (
                    egui::Color32::from_rgb(58, 96, 72),
                    egui::Stroke::new(1.25, egui::Color32::from_rgb(100, 170, 125)),
                    egui::Color32::from_rgb(225, 245, 232),
                )
            } else {
                (
                    ui.visuals().extreme_bg_color,
                    egui::Stroke::new(1.0, ui.visuals().widgets.inactive.fg_stroke.color),
                    ui.visuals().widgets.inactive.fg_stroke.color,
                )
            };
            let button_response = ui.add_enabled(
                enabled,
                egui::Button::new(
                    egui::RichText::new(format!("{}. {}", index + 1, step.title()))
                        .color(text_color)
                        .strong(),
                )
                .selected(selected)
                .fill(fill)
                .stroke(stroke)
                .min_size(egui::vec2(180.0, 42.0)),
            );

            if button_response.clicked() {
                *current_step = step;
            }

            if index + 1 < steps.len() {
                let arrow_size = egui::vec2(28.0, 28.0);
                let (arrow_rect, _) = ui.allocate_exact_size(arrow_size, egui::Sense::hover());
                let arrow_color = if completed {
                    egui::Color32::from_rgb(100, 170, 125)
                } else if selected {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().widgets.inactive.fg_stroke.color
                };
                let origin = egui::pos2(arrow_rect.left() + 4.0, arrow_rect.center().y);
                ui.painter().arrow(
                    origin,
                    egui::vec2(20.0, 0.0),
                    egui::Stroke::new(2.5, arrow_color),
                );
            }
        }
    });

    ui.add_space(4.0);
}

impl Progression {
    fn is_after(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::CheckProbableFraud, Self::ImportCsv)
                | (Self::ExportCsvView, Self::ImportCsv)
                | (Self::ExportCsvView, Self::CheckProbableFraud)
        )
    }
}

fn export_reviewed_csv(
    ctx: &egui::Context,
    transactions: &model::data::transactions::Transactions,
    picked_name: Option<&str>,
) {
    let default_name = picked_name
        .and_then(|name| std::path::Path::new(name).file_stem())
        .map(|stem| format!("{}_reviewed.csv", stem.to_string_lossy()))
        .unwrap_or_else(|| "transactions_reviewed.csv".to_owned());

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .save_file()
        {
            let _ = transactions.export_csv(path);
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let csv_content = transactions.export_csv_content();
        let encoded = web_sys::js_sys::encode_uri_component(&csv_content);
        let url = format!("data:text/csv;charset=utf-8,{}", encoded);

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Ok(element) = document.create_element("a") {
                    if let Ok(anchor) = element.dyn_into::<web_sys::HtmlAnchorElement>() {
                        anchor.set_href(&url);
                        anchor.set_download(&default_name);
                        anchor.click();
                    }
                }
            }
        }
    }

    ctx.request_repaint();
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
