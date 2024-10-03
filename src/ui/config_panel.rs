use super::app::EnvoyApp;
use super::time_format::pretty_ellapsed_time;
use eframe::egui::{Button, Color32, DragValue, RichText, TopBottomPanel};
use rfd::FileDialog;
use std::time::{Duration, Instant};

/// Render the configuration panel (top panel in the UI)
/// This panel is the one that implements a large part of the UI that
/// directly interacts with the app itself, including the configuration
/// and run controls. The only other panel that has this level of control is the ecc_panel.
pub fn render_config_panel(app: &mut EnvoyApp, ctx: &eframe::egui::Context) {
    TopBottomPanel::top("Config_Panel").show(ctx, |ui| {
        //Drop down menu
        ui.menu_button(RichText::new("File").size(16.0), |ui| {
            if ui.button(RichText::new("Save").size(14.0)).clicked() {
                if let Some(path) = FileDialog::new()
                    .set_directory(
                        &std::env::current_dir().expect("Couldn't access runtime directory"),
                    )
                    .add_filter("YAML", &["yaml", "yml"])
                    .save_file()
                {
                    app.config.path = path;
                    match app.config.save() {
                        Ok(()) => (),
                        Err(e) => tracing::error!("Could not save Config: {e}"),
                    }
                }
                ui.close_menu();
            }
            if ui.button(RichText::new("Open").size(14.0)).clicked() {
                if let Some(path) = FileDialog::new()
                    .set_directory(
                        &std::env::current_dir().expect("Couldn't access runtime directory"),
                    )
                    .add_filter("YAML", &["yaml", "yml"])
                    .pick_file()
                {
                    match app.config.load(path) {
                        Ok(()) => (),
                        Err(e) => tracing::error!("Could not load Config: {e}"),
                    }
                }
                ui.close_menu();
            }
        });

        // Configuration
        ui.separator();
        ui.label(
            RichText::new("Configuration")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Config File: {}", app.config.path.display())).size(16.0),
            );
            ui.label(RichText::new("Experiment").size(16.0));
            ui.add(
                eframe::egui::widgets::TextEdit::singleline(&mut app.config.experiment)
                    .desired_width(100.0)
                    .margin(eframe::egui::Margin::symmetric(4.0, 4.0)),
            );
            ui.label(RichText::new("Run Number").size(16.0));
            ui.add(DragValue::new(&mut app.config.run_number).speed(1));
            ui.label(RichText::new("Description").size(16.0));
            ui.add(
                eframe::egui::widgets::TextEdit::singleline(&mut app.config.description)
                    .desired_width(f32::INFINITY)
                    .margin(eframe::egui::Margin::symmetric(4.0, 4.0)),
            );
        });
        eframe::egui::Grid::new("Config grid")
            .min_col_width(100.0)
            .show(ui, |ui| {
                ui.label(RichText::new("VTHGEM(V)").size(16.0));
                ui.add(DragValue::new(&mut app.config.v_thgem).speed(10));
                ui.label(RichText::new("E-Drift(V)").size(16.0));
                ui.add(DragValue::new(&mut app.config.e_drift).speed(10));
                ui.label(RichText::new("VCathode(kV)").size(16.0));
                ui.add(DragValue::new(&mut app.config.v_cathode).speed(10));
                ui.label(RichText::new("E-Trans(V)").size(16.0));
                ui.add(DragValue::new(&mut app.config.e_trans).speed(10));
                ui.label(RichText::new("VMM(V)").size(16.0));
                ui.add(DragValue::new(&mut app.config.v_mm).speed(10));
                ui.label(RichText::new("Magnetic Field(T)").size(16.0));
                ui.add(DragValue::new(&mut app.config.magnetic_field).speed(0.01));
                ui.end_row();

                ui.label(RichText::new("Target Gas").size(16.0));
                ui.text_edit_singleline(&mut app.config.gas);
                ui.label(RichText::new("Pressure(Torr)").size(16.0));
                ui.add(DragValue::new(&mut app.config.pressure).speed(10));
                ui.label(RichText::new("Beam").size(16.0));
                ui.text_edit_singleline(&mut app.config.beam);
                ui.label(RichText::new("Beam Energy (MeV/U)").size(16.0));
                ui.add(DragValue::new(&mut app.config.energy).speed(1));
                ui.label(RichText::new("GET Freq. (MHz)").size(16.0));
                ui.add(DragValue::new(&mut app.config.frequency).speed(1));
                ui.end_row();
            });

        // Connect buttons
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Connect to AT-TPC")
                    .size(16.0)
                    .color(Color32::LIGHT_BLUE),
            );
            if ui
                .add_enabled(
                    !app.embassy.is_running(),
                    Button::new(
                        RichText::new("Connect")
                            .color(Color32::LIGHT_BLUE)
                            .size(16.0),
                    )
                    .min_size([100.0, 25.0].into()),
                )
                .clicked()
            {
                app.connect();
            }
            if ui
                .add_enabled(
                    app.embassy.is_running(),
                    Button::new(
                        RichText::new("Disconnect")
                            .color(Color32::LIGHT_RED)
                            .size(16.0),
                    )
                    .min_size([100.0, 25.0].into()),
                )
                .clicked()
            {
                app.disconnect();
            }
            //Start/Stop
            ui.label(
                RichText::new("Run Control")
                    .size(16.0)
                    .color(Color32::LIGHT_BLUE),
            );
            if ui
                .add_enabled(
                    app.status.is_system_ready(),
                    Button::new(RichText::new("Start").color(Color32::GREEN).size(16.0))
                        .min_size([100.0, 25.0].into()),
                )
                .clicked()
            {
                app.start_run();
            }

            if ui
                .add_enabled(
                    app.status.is_system_running(),
                    Button::new(RichText::new("Stop").color(Color32::RED).size(16.0))
                        .min_size([100.0, 25.0].into()),
                )
                .clicked()
            {
                app.stop_run();
            }

            let mut run_duration = Duration::from_secs(0);
            if app.status.is_system_running() {
                run_duration = Instant::now() - app.run_start_time;
            }
            ui.label(
                RichText::new(format!(
                    "Duration(hrs:mins:ss): {}",
                    pretty_ellapsed_time(run_duration.as_secs())
                ))
                .size(16.0)
                .color(Color32::LIGHT_BLUE),
            );
        });
        ui.separator();
    });
}
