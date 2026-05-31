#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod csv_loader;
mod state;
mod transactions_table;

pub use app::TemplateApp;
