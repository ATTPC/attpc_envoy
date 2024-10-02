use super::app::EnvoyApp;
use crate::envoy::surveyor_status::{SurveyorDiskStatus, SurveyorStatus};
use eframe::egui::{CentralPanel, Color32, RichText};

/// Render the panel displaying data router status, this is the central panel in the UI
pub fn render_data_router_panel(app: &mut EnvoyApp, ctx: &eframe::egui::Context) {
    CentralPanel::default().show(ctx, |ui| {
        let surv_system_stat = app.status.get_surveyor_system_status();
        ui.label(
            RichText::new("Data Router Status")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );
        ui.label(
            RichText::new(format!("System Status: {}", surv_system_stat))
                .color(&surv_system_stat)
                .size(16.0),
        );
        ui.separator();
        ui.label(RichText::new("Status Board").size(16.0));
        ui.separator();
        ui.push_id(1, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .column(egui_extras::Column::auto().at_least(90.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(50.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(150.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(100.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(50.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(120.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(140.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(100.0).resizable(true))
                .column(egui_extras::Column::auto().at_least(100.0).resizable(true))
                .header(40.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Envoy");
                    });
                    header.col(|ui| {
                        ui.heading("Status");
                    });
                    header.col(|ui| {
                        ui.heading("Location");
                    });
                    header.col(|ui| {
                        ui.heading("Disk Status");
                    });
                    header.col(|ui| {
                        ui.heading("Files");
                    });
                    header.col(|ui| {
                        ui.heading("Bytes Written");
                    });
                    header.col(|ui| {
                        ui.heading("Data Rate (MB/s)");
                    });
                    header.col(|ui| {
                        ui.heading("%Disk Used");
                    });
                    header.col(|ui| {
                        ui.heading("Disk Size");
                    });
                })
                .body(|body| {
                    let surveyor_status = app.status.get_surveyor_status_response();
                    body.rows(40.0, surveyor_status.len(), |mut row| {
                        let ridx = row.index();
                        let status = &surveyor_status[ridx];
                        let disk_stat = SurveyorDiskStatus::from(status.disk_status.as_str());
                        row.col(|ui| {
                            ui.label(
                                RichText::new(format!("Data Router {}", ridx))
                                    .color(Color32::LIGHT_GREEN),
                            );
                        });
                        row.col(|ui| {
                            let surv_type = SurveyorStatus::from(status.state);
                            ui.label(RichText::new(format!("{}", surv_type)).color(&surv_type));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(status.location.clone()));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{}", disk_stat)).color(&disk_stat));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{}", status.files)));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(human_bytes::human_bytes(
                                status.bytes_used as f64,
                            )));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{:.3}", status.data_rate)));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(status.percent_used.clone()));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(human_bytes::human_bytes(
                                status.disk_space as f64,
                            )));
                        });
                    })
                });
        });

        ui.separator();
    });
}
