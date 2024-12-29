use super::app::EnvoyApp;
use crate::envoy::sentry_types::SentryServerStatus;
use eframe::egui::{CentralPanel, Color32, RichText};

/// Render the panel displaying data router status, this is the central panel in the UI
pub fn render_sentry_panel(app: &mut EnvoyApp, ctx: &eframe::egui::Context) {
    CentralPanel::default().show(ctx, |ui| {
        let sentry_system_stat = app.status.get_sentry_server_system_status();
        ui.label(
            RichText::new("Sentry Status")
                .color(Color32::LIGHT_BLUE)
                .size(18.0),
        );
        ui.label(
            RichText::new(format!("System Status: {}", sentry_system_stat))
                .color(&sentry_system_stat)
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
                        ui.heading("Data Path");
                    });
                    header.col(|ui| {
                        ui.heading("Process");
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
                    let sentry_status = app.status.get_sentry_status_response();
                    body.rows(40.0, sentry_status.len(), |mut row| {
                        let ridx = row.index();
                        let status = &sentry_status[ridx];
                        let server_stat = SentryServerStatus::from(status);
                        row.col(|ui| {
                            ui.label(
                                RichText::new(format!("Sentry {}", ridx))
                                    .color(Color32::LIGHT_GREEN),
                            );
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{}", server_stat)).color(&server_stat));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(status.data_path.clone()));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{}", status.process.clone())));
                        });
                        row.col(|ui| {
                            ui.label(
                                RichText::new(format!("{}", status.data_path_files))
                                    .color(super::style::n_files_color(&status.data_path_files)),
                            );
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(human_bytes::human_bytes(
                                status.data_written_gb * 1.0e9,
                            )));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{:.3}", status.data_rate_mb)));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(format!(
                                "{:3}%",
                                1.0 - status.disk_avail_gb / status.disk_total_gb,
                            )));
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(human_bytes::human_bytes(
                                status.disk_total_gb * 1.0e9,
                            )));
                        });
                    })
                });
        });

        ui.separator();
    });
}
