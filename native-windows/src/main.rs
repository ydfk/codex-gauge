#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod codex;
mod config;
mod model;
mod presentation;
mod storage;
mod updater;
mod windows;

slint::include_modules!();

fn main() {
    if updater::run_replacement_helper() {
        return;
    }
    windows::initialize_dpi_awareness();
    if let Err(error) = app::run() {
        eprintln!("application_error={}", error);
    }
}
