use super::app::EnvoyApp;
use crate::envoy::constants::MUTANT_ID;
use crate::envoy::ecc_operation::ECCStatus;
use crate::envoy::transition::{backward_transition_all, forward_transition_all, transition_ecc};
use eframe::egui::{Button, Color32, RichText, SidePanel};

/// Render the ECC envoy control panel, the left side panel in the ui
pub fn render_ecc_panel(app: &mut EnvoyApp, ctx: &eframe::egui::Context) {
    SidePanel::left("ECC_Panel").show(ctx, |ui| {
        ui.label(
            RichText::new("ECC Envoy Status/Control")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );
        let ecc_system_stat = app.status.get_system_ecc_status();
        ui.label(
            RichText::new(format!("System Status: {}", ecc_system_stat))
                .size(16.0)
                .color(&ecc_system_stat),
        );
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Regress system").size(16.0));
            if ui
                .add_enabled(
                    app.status.get_system_ecc_status().can_go_backward(),
                    Button::new(RichText::new("\u{25C0}").color(Color32::RED).size(16.0)),
                )
                .clicked()
            {
                backward_transition_all(&mut app.embassy, &mut app.status);
            }
            ui.label(RichText::new("Progress system").size(16.0));
            if ui
                .add_enabled(
                    app.status.get_system_ecc_status().can_go_forward(),
                    Button::new(RichText::new("\u{25B6}").color(Color32::GREEN).size(16.0)),
                )
                .clicked()
            {
                match forward_transition_all(&mut app.embassy, &mut app.status) {
                    Ok(()) => (),
                    Err(e) => tracing::error!(
                        "An error occurred attempting to transition the system state: {}",
                        e
                    ),
                }
            }
        });
        ui.separator();

        let mut forward_transitions: Vec<usize> = vec![];
        let mut backward_transitions: Vec<usize> = vec![];

        ui.push_id(0, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .column(egui_extras::Column::auto().at_least(150.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(100.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(50.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(50.0).resizable(true))
                .header(40.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Envoy");
                    });
                    header.col(|ui| {
                        ui.heading("Status");
                    });
                    header.col(|ui| {
                        ui.heading("Regress");
                    });
                    header.col(|ui| {
                        ui.heading("Progress");
                    });
                })
                .body(|body| {
                    let ecc_status = app.status.get_ecc_status_response();
                    body.rows(40.0, ecc_status.len(), |mut row| {
                        let ridx = row.index();
                        let status = &ecc_status[ridx];
                        let ecc_type = ECCStatus::from(status.state);
                        row.col(|ui| {
                            if ridx == MUTANT_ID {
                                ui.label(
                                    RichText::new(format!("ECC Envoy {} [MuTaNT]", ridx))
                                        .color(Color32::LIGHT_GREEN),
                                );
                            } else {
                                ui.label(
                                    RichText::new(format!("ECC Envoy {} [CoBo]", ridx))
                                        .color(Color32::LIGHT_GREEN),
                                );
                            }
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{}", ecc_type)).color(&ecc_type));
                        });
                        row.col(|ui| {
                            if ui
                                .add_enabled(
                                    ecc_type.can_go_backward(),
                                    Button::new(RichText::new("\u{25C0}").color(Color32::RED)),
                                )
                                .clicked()
                            {
                                backward_transitions.push(ridx);
                            }
                        });
                        row.col(|ui| {
                            if ui
                                .add_enabled(
                                    app.status.can_ecc_go_forward(ridx),
                                    Button::new(RichText::new("\u{25B6}").color(Color32::GREEN)),
                                )
                                .clicked()
                            {
                                forward_transitions.push(ridx);
                            }
                        });
                    });
                });
            ui.separator();
        });
        transition_ecc(&mut app.embassy, &mut app.status, forward_transitions, true);
        transition_ecc(
            &mut app.embassy,
            &mut app.status,
            backward_transitions,
            false,
        );
    });
}
