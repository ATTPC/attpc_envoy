use super::config::Config;
use super::graph_manager::GraphManager;
use super::status_manager::StatusManager;
use crate::envoy::embassy::{Embassy, connect_embassy};
use crate::envoy::message::EmbassyMessage;
use crate::envoy::ecc_operation::{ECCStatus, ECCOperation};
use crate::envoy::surveyor_state::{SurveyorState, SurveyorDiskStatus};
use crate::envoy::constants::{NUMBER_OF_MODULES, MUTANT_ID};
use crate::command::command::{execute, CommandName, CommandStatus};

use std::path::Path;
use std::fs::File;
use std::io::{Read, Write};
use eframe::egui::{RichText, Color32};
use eframe::egui::widgets::Button;

const DEFAULT_TEXT_COLOR: Color32 = Color32::LIGHT_GRAY;

/// # EnvoyApp
/// EnvoyApp implements the eframe::App trait, and holds most of the controlling functionality of the program,
/// including the tokio runtime and the embassy hub.
#[derive(Debug)]
pub struct EnvoyApp {
    config: Config,
    runtime: tokio::runtime::Runtime,
    embassy: Option<Embassy>,
    envoy_handles: Option<Vec<tokio::task::JoinHandle<()>>>,
    status: StatusManager,
    graphs: GraphManager,
    max_graph_points: usize
}

impl EnvoyApp {
    /// Create an app from a tokio runtime and eframe context
    pub fn new(cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        let mut visuals = eframe::egui::Visuals::dark();
        visuals.override_text_color = Some(DEFAULT_TEXT_COLOR);
        cc.egui_ctx.set_visuals(visuals);
        EnvoyApp { config: Config::new(), runtime, embassy: None, envoy_handles: None, status: StatusManager::new(), graphs: GraphManager::new(10), max_graph_points: 10 }
    }

    /// Read in a config from a YAML file at the filepath
    fn read_config(&mut self, filepath: &Path) {
        if let Ok(mut file) = File::open(filepath) {
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
            }
        } else {
            tracing::error!("Could not open the selected file!");
        }
    }

    /// Write the current config to a YAML file at the filepath
    fn write_config(&mut self, filepath: &Path) {
        if let Ok(mut file) = File::create(filepath) {
            let yaml_str = match serde_yaml::to_string::<Config>(&self.config) {
                Ok(yaml) => yaml,
                Err(e) => {
                    tracing::error!("Could not convert config to yaml: {}", e);
                    return;
                }
            };
            match file.write(yaml_str.as_bytes()) {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("Could not write yaml file: {}", e);
                    return;
                }
            }
        }
    }

    /// Create all of the envoys, the embassy, and start the async tasks
    fn connect(&mut self) {
        if self.embassy.is_none() && self.envoy_handles.is_none() {
            let (em, handles) = connect_embassy(&mut self.runtime, &self.config.experiment);
            tracing::info!("Connnected with {} tasks spawned", handles.len());
            self.embassy = Some(em);
            self.envoy_handles = Some(handles);
        }
    }

    /// Emit a cancel signal to all of the envoys and destroy the envoys and the embassy
    /// This can cause a small blocking period while waiting for all of the tasks to join back.
    fn disconnect(&mut self) {
        if self.embassy.is_some() {
            let mut embassy = self.embassy.take().expect("Literally cant happen");
            embassy.shutdown();
            let handles = self.envoy_handles.take().expect("Handles did not exist at disconnect?");
            for handle in handles {
                match self.runtime.block_on(handle) {
                    Ok(()) => (),
                    Err(e) => tracing::error!("Encountered an error whilst disconnecting: {}", e)
                }
            }
            tracing::info!("Disconnected the embassy");
            self.status.reset();
            tracing::info!("Status manager reset.")
        }
    }

    /// Read and handle any messages the embassy recieved from the envoys. Messages are sent
    /// to observer like structures (the StatusManager and GraphManager)
    fn poll_embassy(&mut self) {
        if let Some(embassy) = self.embassy.as_mut() {
            match embassy.poll_messages() {
                Ok(messages) => {
                    match self.status.handle_messages(&messages) {
                        Ok(_) => (),
                        Err(e) => tracing::error!("StatusManager ran into an error handling messages: {}", e)
                    };
                    match self.graphs.handle_messages(&messages) {
                        Ok(_) => (),
                        Err(e) => tracing::error!("GraphManager ran into an error handling messages: {}", e)
                    }
                }
                Err(e) => tracing::error!("Embassy ran into an error polling the envoys: {}", e)
            };
        }
    }

    /// Send a transition command to some of the ECC operation envoys. Transitions are either forward or backward 
    /// depending on the is_forward flag. What type of transition is determined by the current state of the envoy as last recorded
    /// by the status envoy.
    fn transition_ecc(&mut self, ids: Vec<usize>, is_forward: bool) {
        if ids.len() == 0 {
            return;
        }

        if self.embassy.is_none() {
            tracing::error!("Some how trying to operate on ECC whilst disconnected!");
            return;
        }
        for id in ids {
            let status = &self.status.get_ecc_status()[id];
            let operation: ECCOperation;
            if is_forward {
                operation = ECCStatus::from(status.state).get_forward_operation();
            
            } else {
                operation = ECCStatus::from(status.state).get_backward_operation();
            }
            match operation {
                ECCOperation::Invalid => (),
                _ => {
                    match self.embassy.as_mut().unwrap().submit_message(EmbassyMessage::compose_ecc_op(operation.into(), id as i32)) {
                        Ok(()) => (),
                        Err(e) => tracing::error!("Embassy had an error sending a message: {}", e)
                    }
                }
            }
            self.status.set_ecc_status_transition(id);
        }
    }

    /// Transition all of the envoys forward (Progress)
    fn forward_transition_all(&mut self) {
        let ids: Vec<usize> = (0..(NUMBER_OF_MODULES as usize)).collect();
        self.transition_ecc(ids, true)
    }

    /// Transition all of the envoys backward (Regress)
    fn backward_transition_all(&mut self) {
        let ids: Vec<usize> = (0..(NUMBER_OF_MODULES as usize)).collect();
        self.transition_ecc(ids, false)
    }

    /// Send a start run command to all of the envoys.
    /// Note that several important things must happen here. First a command is sent to make sure that 
    /// the run number was not already used. Then, the CoBos must start, and only once all CoBos are running,
    /// does the Mutant start. The rate graphs are also reset.
    fn start_run(&mut self) {
        //Order is all cobos, then mutant
        let operation = ECCOperation::Start;
        self.graphs.reset_graphs();

        //Check the run number status using the shell scripting engine
        match execute(CommandName::CheckRunExists, self.status.get_surveyor_status(), &self.config.experiment, &self.config.run_number) {
            CommandStatus::Success => {
                tracing::warn!("Tried to start a run with a run number that was already used! Either delete the extant data or change the run number!");
                return;
            }
            CommandStatus::Failure => (),
            CommandStatus::CouldNotExecute => return
        }

        //Start CoBos
        for id in 0..(NUMBER_OF_MODULES-1) {
            match self.embassy.as_mut().unwrap().submit_message(EmbassyMessage::compose_ecc_op(operation.clone().into(), id)) {
                Ok(()) => (),
                Err(e) => tracing::error!("Embassy had an error sending a start run message: {}", e)
            }
        }

        //Wait for good CoBo status
        loop {
            if self.status.is_all_but_mutant_running() {
                break;
            }
        }

        //Start mutant
        match self.embassy.as_mut().unwrap().submit_message(EmbassyMessage::compose_ecc_op(operation.clone().into(), MUTANT_ID)) {
            Ok(()) => (),
            Err(e) => tracing::error!("Embassy had an error sending a start run message: {}", e)
        }
    }

    /// Send a stop run command to all of the envoys.
    /// Note that several important things must happen here. First the Mutant is stopped. Then, only after the Mutant has stopped,
    /// all of the Cobos are told to stop. After the stop command is issued, a command is sent to move all of the data to a run specific location,
    /// as well as a command to back up the ECC configuration files.
    fn stop_run(&mut self) {
        //Order is mutant, all cobos
        let operation = ECCOperation::Stop;

        //Stop the mutant
        match self.embassy.as_mut().unwrap().submit_message(EmbassyMessage::compose_ecc_op(operation.clone().into(), MUTANT_ID)) {
            Ok(()) => (),
            Err(e) => tracing::error!("Embassy had an error sending a stop run message: {}", e)
        }

        //Wait for mutant to stop 
        loop {
            if self.status.is_mutant_stopped() {
                break;
            }
        }

        //Stop all of the CoBos
        for id in 0..(NUMBER_OF_MODULES-1) {
            match self.embassy.as_mut().unwrap().submit_message(EmbassyMessage::compose_ecc_op(operation.clone().into(), id)) {
                Ok(()) => (),
                Err(e) => tracing::error!("Embassy had an error sending a start run message: {}", e)
            }
        }

        match execute(CommandName::MoveGrawFiles, self.status.get_surveyor_status(), &self.config.experiment, &self.config.run_number) {
            CommandStatus::Success => (),
            CommandStatus::Failure => tracing::error!("Unable to move the graw files after the stop run signal!"),
            CommandStatus::CouldNotExecute => ()
        }

        match execute(CommandName::BackupConfig, self.status.get_surveyor_status(), &self.config.experiment, &self.config.run_number) {
            CommandStatus::Success => (),
            CommandStatus::Failure => tracing::error!("Could not backup config files after the stop run signal"),
            CommandStatus::CouldNotExecute => (),
        }

        self.config.run_number += 1;
    }
}

impl eframe::App for EnvoyApp {

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {

        //Probably don't want to poll every frame, but as a test...
        self.poll_embassy();

        // The top panel, contains the specific configuration
        eframe::egui::TopBottomPanel::top("Config_Panel")
        .show(ctx, |ui| {

            ui.menu_button(RichText::new("File").size(16.0), |ui| {
                if ui.button(RichText::new("Save").size(14.0)).clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(&std::env::current_dir().expect("Couldn't access runtime directory"))
                        .add_filter("YAML file", &["yaml"])
                        .show_save_single_file()
                    {
                        self.write_config(&path);
                    }
                    ui.close_menu();
                }
                if ui.button(RichText::new("Open").size(14.0)).clicked() {
                    if let Ok(Some(path)) = native_dialog::FileDialog::new()
                        .set_location(&std::env::current_dir().expect("Couldn't access runtime directory"))
                        .add_filter("YAML file", &["yaml"])
                        .show_open_single_file()
                    {
                        self.read_config(&path);
                    }
                    ui.close_menu();
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.add_enabled(self.embassy.is_none(), Button::new(RichText::new("Connect").color(Color32::LIGHT_BLUE).size(16.0)).min_size([100.0, 25.0].into())).clicked() {
                    self.connect();
                }
                if ui.add_enabled(self.embassy.is_some(), Button::new(RichText::new("Disconnect").color(Color32::LIGHT_RED).size(16.0)).min_size([100.0, 25.0].into())).clicked() {
                    self.disconnect();
                }
            });

            ui.separator();
            ui.label(RichText::new("Configuration").color(Color32::LIGHT_BLUE).size(18.0));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Experiment").size(16.0));
                ui.text_edit_singleline(&mut self.config.experiment);
            });
            
            ui.horizontal(|ui| {
                ui.label(RichText::new("Description").size(16.0));
                ui.text_edit_singleline(&mut self.config.description);
            });
            
            ui.horizontal(|ui| {
                ui.label(RichText::new("Run Number").size(16.0));
                ui.add(eframe::egui::widgets::DragValue::new(&mut self.config.run_number).speed(1));
            });
            ui.separator();

            ui.horizontal(|ui| {
                if ui.add_enabled(self.status.is_system_ready(), Button::new(RichText::new("Start").color(Color32::GREEN).size(16.0)).min_size([100.0, 25.0].into())).clicked() {
                    self.start_run();
                }
    
                if ui.add_enabled(self.status.is_system_running(), Button::new(RichText::new("Stop").color(Color32::RED).size(16.0)).min_size([100.0, 25.0].into())).clicked() {
                    self.stop_run();
                }
            });
            ui.separator();
        });

        // Bottom panel, contains the rate graph
        eframe::egui::TopBottomPanel::bottom("Graph_Panel").show(ctx, |ui| {
            ui.separator();
            let lines = self.graphs.get_line_graphs();
            ui.label(RichText::new("Data Rate Graph").color(Color32::LIGHT_BLUE).size(18.0));
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
                for line  in lines {
                    plot_ui.line(line);
                }
            });
            ui.separator();
        });

        //Side panel showing all ECC Envoy controls
        eframe::egui::SidePanel::left("ECC_Panel")
        .show(ctx, |ui| {
            ui.label(RichText::new("ECC Envoy Status/Control").color(Color32::LIGHT_BLUE).size(18.0));
            let ecc_system_stat = self.status.get_system_ecc_status();
            ui.label(RichText::new(format!("System Status: {}", ecc_system_stat)).size(16.0).color(&ecc_system_stat));
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Regress system").size(16.0));
                if ui.add_enabled(self.status.get_system_ecc_status().can_go_backward(), Button::new(RichText::new("\u{25C0}").color(Color32::RED).size(16.0))).clicked() {
                    self.backward_transition_all();
                }
                ui.label(RichText::new("Progress system").size(16.0));
                if ui.add_enabled(self.status.get_system_ecc_status().can_go_forward(),Button::new(RichText::new("\u{25B6}").color(Color32::GREEN).size(16.0))).clicked() {
                    self.forward_transition_all();
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
                        let ecc_status = self.status.get_ecc_status();
                        body.rows(40.0, ecc_status.len(), |ridx, mut row| {
                            let status = &ecc_status[ridx];
                            let ecc_type = ECCStatus::from(status.state);
                            row.col(|ui| {
                                if (ridx as i32) == MUTANT_ID {
                                    ui.label(RichText::new(format!("ECC Envoy {} [MuTaNT]", ridx)).color(Color32::LIGHT_GREEN));
                                } else {
                                    ui.label(RichText::new(format!("ECC Envoy {} [CoBo]", ridx)).color(Color32::LIGHT_GREEN));
                                }
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(format!("{}", ecc_type)).color(&ecc_type));
                            });
                            row.col(|ui| {
                                if ui.add_enabled(ecc_type.can_go_backward(), Button::new(RichText::new("\u{25C0}").color(Color32::RED))).clicked() {
                                    forward_transitions.push(ridx);
                                }
                            });
                            row.col(|ui| {
                                if ui.add_enabled(ecc_type.can_go_forward(), Button::new(RichText::new("\u{25B6}").color(Color32::GREEN))).clicked() {
                                    backward_transitions.push(ridx);
                                }
                            });
                        });
                    });
                ui.separator();
            });
            self.transition_ecc(forward_transitions, true);
            self.transition_ecc(backward_transitions, false);
        });

        //Central panel showing Data router info. Use central to allow for dynamic resizing of the window.
        eframe::egui::CentralPanel::default()
        .show(ctx,|ui| {
            let surv_system_stat = self.status.get_surveyor_system_status();
            ui.label(RichText::new("Data Router Status").color(Color32::LIGHT_BLUE).size(18.0));
            ui.label(RichText::new(format!("System Status: {}", surv_system_stat)).color(&surv_system_stat).size(16.0));
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
                        let surveyor_status = self.status.get_surveyor_status();
                        body.rows(40.0, surveyor_status.len(), |ridx, mut row| {
                            let status = &surveyor_status[ridx];
                            let disk_stat = SurveyorDiskStatus::from(status.disk_status.as_str());
                            row.col(|ui| {
                                ui.label(RichText::new(format!("Data Router {}", ridx)).color(Color32::LIGHT_GREEN));
                            });
                            row.col(|ui| {
                                let surv_type = SurveyorState::from(status.state);
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
                                ui.label(RichText::new(format!("{}", human_bytes::human_bytes(status.bytes_used as f64))));
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(format!("{}", status.data_rate)));
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(status.percent_used.clone()));
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(format!("{}", human_bytes::human_bytes(status.disk_space as f64))));
                            });
                        })
                    });
            });
                
            ui.separator();
        });
        
        

        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}