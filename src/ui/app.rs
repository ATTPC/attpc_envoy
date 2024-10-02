use super::config::Config;
use super::graph_manager::GraphManager;
use super::time_format::pretty_ellapsed_time;
use crate::command::bash_command::{execute, CommandName, CommandStatus};
use crate::envoy::constants::MUTANT_ID;
use crate::envoy::ecc_operation::ECCStatus;
use crate::envoy::embassy::Embassy;
use crate::envoy::status_manager::StatusManager;
use crate::envoy::surveyor_status::{SurveyorDiskStatus, SurveyorStatus};
use crate::envoy::transition::*;

use eframe::egui::widgets::Button;
use eframe::egui::widgets::DragValue;
use eframe::egui::{CentralPanel, SidePanel, TopBottomPanel};
use eframe::egui::{Color32, RichText};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DEFAULT_TEXT_COLOR: Color32 = Color32::LIGHT_GRAY;

/// EnvoyApp implements the eframe::App trait,
/// and holds the tokio runtime and the embassy hub.
#[derive(Debug)]
pub struct EnvoyApp {
    config: Config,
    embassy: Embassy,
    status: StatusManager,
    graphs: GraphManager,
    max_graph_points: usize,
    run_start_time: Instant,
    run_duration: Duration,
}

//*************//
// STATE LOGIC //
//*************//
impl EnvoyApp {
    /// Create an app from a tokio runtime and eframe context
    pub fn new(cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let mut visuals = eframe::egui::Visuals::dark();
        visuals.override_text_color = Some(DEFAULT_TEXT_COLOR);
        cc.egui_ctx.set_visuals(visuals);
        EnvoyApp {
            config: Config::new(),
            embassy: Embassy::new(runtime),
            status: StatusManager::new(),
            graphs: GraphManager::new(10, 2),
            max_graph_points: 10,
            run_start_time: Instant::now(),
            run_duration: Duration::from_secs(0),
        }
    }

    /// Read in a config from a YAML file at the filepath
    fn read_config(&mut self, filepath: PathBuf) {
        if let Ok(mut file) = File::open(&filepath) {
            let mut yaml_str = String::new();
            match file.read_to_string(&mut yaml_str) {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Could not read yaml file: {}", e);
                    return;
                }
            }
            self.config = match serde_yaml::from_str::<Config>(&yaml_str) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Could not deserialize config: {}", e);
                    return;
                }
            };
            self.config.config_path = filepath;
        } else {
            tracing::error!("Could not open the selected file!");
        }
    }

    /// Write the current config to a YAML file at the filepath
    fn write_config(&self) {
        if let Ok(mut file) = File::create(&self.config.config_path) {
            let yaml_str = match serde_yaml::to_string::<Config>(&self.config) {
                Ok(yaml) => yaml,
                Err(e) => {
                    tracing::error!("Could not convert config to yaml: {}", e);
                    return;
                }
            };
            match file.write_all(yaml_str.as_bytes()) {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Could not write yaml file: {}", e);
                }
            }
        }
    }

    /// Create all of the envoys, the embassy, and start the async tasks
    fn connect(&mut self) {
        if !self.embassy.is_running() {
            self.embassy.startup(&self.config.experiment);
            tracing::info!(
                "Connnected with {} tasks spawned",
                self.embassy.number_of_tasks()
            );
        }
    }

    /// Emit a cancel signal to all of the envoys and destroy the envoys and the embassy
    /// This can cause a small blocking period while waiting for all of the tasks to join back.
    fn disconnect(&mut self) {
        if self.embassy.is_running() {
            match self.embassy.shutdown() {
                Ok(()) => (),
                Err(e) => tracing::error!("Failed to stop the embassy: {e}"),
            }
            self.status.reset();
            tracing::info!("Disconnected the embassy");
            tracing::info!("Status manager reset.")
        }
    }

    /// Send a start run command to all of the envoys.
    /// Note that several important things must happen here. First a command is sent to make sure that
    /// the run number was not already used. Then, the CoBos must start, and only once all CoBos are running,
    /// does the Mutant start. The rate graphs are also reset.
    fn start_run(&mut self) {
        //Order is all cobos, then mutant

        //Check the run number status using the shell scripting engine
        tracing::info!("Starting run {} ...", self.config.run_number);
        tracing::info!("Checking if run number is ok...");
        match execute(
            CommandName::CheckRunExists,
            self.status.get_surveyor_status_response(),
            &self.config.experiment,
            &self.config.run_number,
        ) {
            CommandStatus::Success => {
                tracing::warn!("Tried to start a run with a run number that was already used! Either delete the extant data or change the run number!");
                return;
            }
            CommandStatus::Failure => (),
            CommandStatus::CouldNotExecute => return,
        }
        tracing::info!("Run number validated.");

        tracing::info!("Re-configuring MuTaNT to reset timestamps...");
        match reconfigure_mutant_blocking(&mut self.embassy, &mut self.status) {
            Ok(()) => (),
            Err(e) => tracing::error!("An error occured reconfiguring MuTaNT: {}", e),
        }
        tracing::info!("MuTaNT is re-configured. Proceeding.");

        tracing::info!("Starting CoBos...");
        //Start CoBos
        match start_cobos_blocking(&mut self.embassy, &mut self.status) {
            Ok(()) => (),
            Err(e) => tracing::error!("An error occured starting the CoBos: {}", e),
        }

        tracing::info!("CoBos started.");

        tracing::info!("Starting MuTaNT...");
        match start_mutant(&mut self.embassy) {
            Ok(()) => (),
            Err(e) => tracing::error!("An error occured starting the MuTaNT: {}", e),
        }
        tracing::info!("MuTaNT started.");
        tracing::info!("Run {} successfully started!", self.config.run_number);

        //Update run start time
        self.run_start_time = Instant::now();
        self.graphs.reset();
    }

    /// Send a stop run command to all of the envoys.
    /// Note that several important things must happen here. First the Mutant is stopped. Then, only after the Mutant has stopped,
    /// all of the Cobos are told to stop. After the stop command is issued, a command is sent to move all of the data to a run specific location,
    /// as well as a command to back up the ECC configuration files.
    fn stop_run(&mut self) {
        //Order is mutant, all cobos
        tracing::info!("Stopping run {} ...", self.config.run_number);
        tracing::info!("Stopping the MuTaNT...");
        //Stop the mutant
        match stop_mutant_blocking(&mut self.embassy, &mut self.status) {
            Ok(()) => (),
            Err(e) => tracing::error!("Embassy had an error stopping the MuTaNT: {}", e),
        }

        tracing::info!("MuTaNT stopped.");
        tracing::info!("Stopping CoBos...");

        //Stop all of the CoBos
        match stop_cobos(&mut self.embassy) {
            Ok(()) => (),
            Err(e) => {
                tracing::error!("Embassy had an error stoppging the CoBos: {}", e)
            }
        }

        tracing::info!("CoBos stopped.");
        tracing::info!("Moving .graw files...");

        match execute(
            CommandName::MoveGrawFiles,
            self.status.get_surveyor_status_response(),
            &self.config.experiment,
            &self.config.run_number,
        ) {
            CommandStatus::Success => (),
            CommandStatus::Failure => {
                tracing::error!("Unable to move the graw files after the stop run signal!")
            }
            CommandStatus::CouldNotExecute => (),
        }

        tracing::info!(".graw files moved.");
        tracing::info!("Backing up GET configuration...");

        match execute(
            CommandName::BackupConfig,
            self.status.get_surveyor_status_response(),
            &self.config.experiment,
            &self.config.run_number,
        ) {
            CommandStatus::Success => (),
            CommandStatus::Failure => {
                tracing::error!("Could not backup config files after the stop run signal")
            }
            CommandStatus::CouldNotExecute => (),
        }

        tracing::info!("GET configuration backed up.");
        tracing::info!("Run {} stopped!", self.config.run_number);

        tracing::info!("Saving config to table...");
        self.config
            .write_table(Instant::now() - self.run_start_time);
        tracing::info!("Config saved to table.");

        self.config.run_number += 1;
        self.write_config();
        tracing::info!("Config autosaved to {}", self.config.config_path.display());
    }
}
//*************//
// STATE LOGIC //
//*************//

//*************//
//  APP IMPL  //
//*************//
impl eframe::App for EnvoyApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        //Probably don't want to poll every frame, but as a test...
        match poll_embassy(&mut self.embassy, &mut self.status) {
            Ok(()) => (),
            Err(e) => tracing::error!("An error occurred when polling the embassy: {}", e),
        }
        if self.graphs.should_update() && self.embassy.is_running() {
            self.graphs
                .update(self.status.get_surveyor_status_response());
        }
        self.render_config_panel(ctx);
        self.render_graph_panel(ctx);
        self.render_ecc_panel(ctx);
        self.render_data_router_panel(ctx);
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}
//*************//
//  APP IMPL  //
//*************//

//*************//
// PANELS IMPL //
//*************//
impl EnvoyApp {
    ///Render the configuration panel (top panel in the UI)
    fn render_config_panel(&mut self, ctx: &eframe::egui::Context) {
        TopBottomPanel::top("Config_Panel").show(ctx, |ui| {
            //Drop down menu
            ui.menu_button(RichText::new("File").size(16.0), |ui| {
                if ui.button(RichText::new("Save").size(14.0)).clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(
                            &std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("YAML file", &["yaml"])
                        .show_save_single_file()
                    {
                        self.config.config_path = path;
                        self.write_config();
                    }
                    ui.close_menu();
                }
                if ui.button(RichText::new("Open").size(14.0)).clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(
                            &std::env::current_dir().expect("Couldn't access runtime directory"),
                        )
                        .add_filter("YAML file", &["yaml"])
                        .show_open_single_file()
                    {
                        self.read_config(path);
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
            ui.label(
                RichText::new(format!(
                    "Config File: {}",
                    self.config.config_path.display()
                ))
                .size(16.0)
                .color(Color32::LIGHT_BLUE),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Experiment")
                        .size(16.0)
                        .color(Color32::LIGHT_BLUE),
                );
                ui.add(
                    eframe::egui::widgets::TextEdit::singleline(&mut self.config.experiment)
                        .desired_width(100.0)
                        .margin(eframe::egui::Margin::symmetric(4.0, 4.0)),
                );
            });

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Run Number")
                        .size(16.0)
                        .color(Color32::LIGHT_BLUE),
                );
                ui.add(DragValue::new(&mut self.config.run_number).speed(1));
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Description").size(16.0));
                ui.add(
                    eframe::egui::widgets::TextEdit::singleline(&mut self.config.description)
                        .desired_width(f32::INFINITY)
                        .margin(eframe::egui::Margin::symmetric(4.0, 4.0)),
                );
            });
            eframe::egui::Grid::new("Config grid")
                .min_col_width(100.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("VTHGEM(V)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.v_thgem).speed(10));
                    ui.label(RichText::new("E-Drift(V)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.e_drift).speed(10));
                    ui.label(RichText::new("Gas").size(16.0));
                    ui.text_edit_singleline(&mut self.config.gas);
                    ui.label(RichText::new("Magnetic Field(T)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.magnetic_field).speed(0.01));
                    ui.end_row();

                    ui.label(RichText::new("VCathode(kV)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.v_cathode).speed(10));
                    ui.label(RichText::new("E-Trans(V)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.e_trans).speed(10));
                    ui.label(RichText::new("Beam").size(16.0));
                    ui.text_edit_singleline(&mut self.config.beam);
                    ui.end_row();

                    ui.label(RichText::new("VMM(V)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.v_mm).speed(10));
                    ui.label(RichText::new("Pressure(Torr)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.pressure).speed(10));
                    ui.label(RichText::new("Beam Energy (MeV/U)").size(16.0));
                    ui.add(DragValue::new(&mut self.config.energy).speed(1));
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
                        !self.embassy.is_running(),
                        Button::new(
                            RichText::new("Connect")
                                .color(Color32::LIGHT_BLUE)
                                .size(16.0),
                        )
                        .min_size([100.0, 25.0].into()),
                    )
                    .clicked()
                {
                    self.connect();
                }
                if ui
                    .add_enabled(
                        self.embassy.is_running(),
                        Button::new(
                            RichText::new("Disconnect")
                                .color(Color32::LIGHT_RED)
                                .size(16.0),
                        )
                        .min_size([100.0, 25.0].into()),
                    )
                    .clicked()
                {
                    self.disconnect();
                }
            });

            // Start/Stop buttons
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Run Control")
                        .size(16.0)
                        .color(Color32::LIGHT_BLUE),
                );
                if ui
                    .add_enabled(
                        self.status.is_system_ready(),
                        Button::new(RichText::new("Start").color(Color32::GREEN).size(16.0))
                            .min_size([100.0, 25.0].into()),
                    )
                    .clicked()
                {
                    self.start_run();
                }

                if ui
                    .add_enabled(
                        self.status.is_system_running(),
                        Button::new(RichText::new("Stop").color(Color32::RED).size(16.0))
                            .min_size([100.0, 25.0].into()),
                    )
                    .clicked()
                {
                    self.stop_run();
                }

                if self.status.is_system_running() {
                    self.run_duration = Instant::now() - self.run_start_time;
                }
                ui.label(
                    RichText::new(format!(
                        "Duration(hrs:mins:ss): {}",
                        pretty_ellapsed_time(self.run_duration.as_secs())
                    ))
                    .size(16.0)
                    .color(Color32::LIGHT_BLUE),
                );
            });
            ui.separator();
        });
    }

    ///Render the graph panel, the bottom of the UI
    fn render_graph_panel(&mut self, ctx: &eframe::egui::Context) {
        TopBottomPanel::bottom("Graph_Panel").show(ctx, |ui| {
            ui.separator();
            let lines = self.graphs.get_line_graphs();
            ui.label(
                RichText::new("Data Rate Graph")
                    .color(Color32::LIGHT_BLUE)
                    .size(18.0),
            );
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Number of Points Per Graph").size(16.0));
                ui.add(eframe::egui::DragValue::new(&mut self.max_graph_points).speed(1));
            });
            ui.separator();
            if *self.graphs.get_max_points() != self.max_graph_points {
                self.graphs.set_max_points(&self.max_graph_points)
            }
            egui_plot::Plot::new("RatePlot")
                .view_aspect(6.0)
                .height(200.0)
                .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop))
                .x_axis_label(RichText::new("Time Since Last Update (s)").size(16.0))
                .y_axis_label(RichText::new("Rate (MB/s)").size(16.0))
                .show(ui, |plot_ui| {
                    for line in lines {
                        plot_ui.line(line);
                    }
                });
            ui.separator();
        });
    }

    /// Render the ECC envoy control panel, the left side panel in the ui
    fn render_ecc_panel(&mut self, ctx: &eframe::egui::Context) {
        SidePanel::left("ECC_Panel").show(ctx, |ui| {
            ui.label(
                RichText::new("ECC Envoy Status/Control")
                    .color(Color32::LIGHT_BLUE)
                    .size(18.0),
            );
            let ecc_system_stat = self.status.get_system_ecc_status();
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
                        self.status.get_system_ecc_status().can_go_backward(),
                        Button::new(RichText::new("\u{25C0}").color(Color32::RED).size(16.0)),
                    )
                    .clicked()
                {
                    backward_transition_all(&mut self.embassy, &mut self.status);
                }
                ui.label(RichText::new("Progress system").size(16.0));
                if ui
                    .add_enabled(
                        self.status.get_system_ecc_status().can_go_forward(),
                        Button::new(RichText::new("\u{25B6}").color(Color32::GREEN).size(16.0)),
                    )
                    .clicked()
                {
                    match forward_transition_all(&mut self.embassy, &mut self.status) {
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
                        let ecc_status = self.status.get_ecc_status_response();
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
                                        self.status.can_ecc_go_forward(ridx),
                                        Button::new(
                                            RichText::new("\u{25B6}").color(Color32::GREEN),
                                        ),
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
            transition_ecc(
                &mut self.embassy,
                &mut self.status,
                forward_transitions,
                true,
            );
            transition_ecc(
                &mut self.embassy,
                &mut self.status,
                backward_transitions,
                false,
            );
        });
    }

    /// Render the panel displaying data router status, this is the central panel in the UI
    fn render_data_router_panel(&mut self, ctx: &eframe::egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            let surv_system_stat = self.status.get_surveyor_system_status();
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
                        let surveyor_status = self.status.get_surveyor_status_response();
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
}
