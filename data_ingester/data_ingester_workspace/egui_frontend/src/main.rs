#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::time::Instant;

use eframe::egui;
use use_case_runner::{DataCollections, DataType, DataTypeTrait, UseCase, UseCaseRunner};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let mut use_case_runner = use_case_runner::UseCaseRunner::new();

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Data Types");
            for (_name, mut data_source) in use_case_runner.data_collectors.collections.iter_mut() {
                display_data_type(ui, data_source);
            }

            ui.heading("Use Cases");
            for mut use_case in use_case_runner.use_cases.iter_mut() {
                display_use_case(ui, use_case, &use_case_runner.data_collectors);
            }
        });
    })
}

fn display_data_type(ui: &mut egui::Ui, data_source: &mut DataType) {
    ui.horizontal(|ui| {
        let name_label = ui.label("Name: ");
        ui.text_edit_singleline(&mut data_source.name)
            .labelled_by(name_label.id);
    });

    ui.horizontal(|ui| {
        let last_updated_label = ui.label("Last updated (seconds ago): ");
        let mut last_updated_text = data_source
            .last_updated
            .map(|instant| instant.elapsed().as_secs().to_string())
            .unwrap_or_else(|| "Not run".into());
        ui.text_edit_singleline(&mut last_updated_text)
            .labelled_by(last_updated_label.id);
    });
    if ui.button("Update data Source").clicked() {
        data_source.update();
    }
}

fn display_use_case(
    ui: &mut egui::Ui,
    use_case: &mut Box<dyn UseCase>,
    data_collections: &DataCollections,
) {
    ui.horizontal(|ui| {
        let name_label = ui.label("Name: ");
        ui.text_edit_singleline(&mut use_case.name())
            .labelled_by(name_label.id);
    });

    ui.horizontal(|ui| {
        let last_updated_label = ui.label("Last updated (seconds ago): ");
        let mut last_run_text = use_case
            .last_run_instant()
            .map(|instant| instant.elapsed().as_secs().to_string())
            .unwrap_or_else(|| "Not run".into());
        ui.text_edit_singleline(&mut last_run_text)
            .labelled_by(last_updated_label.id);
    });

    ui.horizontal(|ui| {
        let last_updated_label = ui.label("Last Result: ");
        let mut result = use_case
            .result()
            .map(|result| result.status().to_string())
            .unwrap_or_else(|| "No result".into());
        ui.text_edit_singleline(&mut result)
            .labelled_by(last_updated_label.id);
    });

    if ui.button("Run use case").clicked() {
        use_case.run(data_collections);
    }
}
