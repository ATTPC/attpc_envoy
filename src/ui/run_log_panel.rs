use super::app::EnvoyApp;
use eframe::egui::{Color32, Grid, RichText};

pub fn render_run_log_panel(app: &mut EnvoyApp, ctx: &eframe::egui::Context) {
    eframe::egui::SidePanel::left("Run Log Panel").show(ctx, |ui| {
        ui.label(
            RichText::new("Run Log")
                .size(18.0)
                .color(Color32::LIGHT_BLUE),
        );
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Add Field").size(16.0)).clicked() {
                app.config
                    .add_field(app.new_field_name.clone(), String::default());
            }
            ui.text_edit_singleline(&mut app.new_field_name);
        });
        ui.separator();
        Grid::new("Runlog Grid").num_columns(2).show(ui, |ui| {
            for (field, value) in app.config.fields.iter_mut() {
                ui.label(RichText::new(field).size(16.0));
                ui.text_edit_singleline(value);
                ui.end_row();
            }
        });
    });
}
