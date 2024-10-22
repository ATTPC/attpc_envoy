use super::config::Config;
use super::config_panel::render_config_panel;
use super::ecc_panel::render_ecc_panel;
use super::graph_manager::GraphManager;
use super::graph_panel::render_graph_panel;
use super::router_panel::render_data_router_panel;
use crate::command::bash_command::{execute, CommandName, CommandStatus};
use crate::envoy::embassy::Embassy;
use crate::envoy::status_manager::StatusManager;
use crate::envoy::transition::*;

use eframe::egui::Color32;
use std::time::Instant;

const DEFAULT_TEXT_COLOR: Color32 = Color32::LIGHT_GRAY;

/// EnvoyApp implements the eframe::App trait,
/// and holds the tokio runtime and the embassy hub.
#[derive(Debug)]
pub struct EnvoyApp {
    pub config: Config,
    pub embassy: Embassy,
    pub status: StatusManager,
    pub graphs: GraphManager,
    pub run_start_time: Instant,
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
        cc.egui_ctx.set_theme(eframe::egui::Theme::Dark);
        EnvoyApp {
            config: Config::new(),
            embassy: Embassy::new(runtime),
            status: StatusManager::new(),
            graphs: GraphManager::new(10, 2),
            run_start_time: Instant::now(),
        }
    }

    /// Create all of the envoys, the embassy, and start the async tasks
    pub fn connect(&mut self) {
        if !self.embassy.is_connected() {
            self.embassy.startup(&self.config.experiment);
            tracing::info!(
                "Connnected with {} tasks spawned",
                self.embassy.number_of_tasks()
            );
        }
    }

    /// Emit a cancel signal to all of the envoys and destroy the envoys and the embassy
    /// This can cause a small blocking period while waiting for all of the tasks to join back.
    pub fn disconnect(&mut self) {
        if self.embassy.is_connected() {
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
    pub fn start_run(&mut self) {
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
    pub fn stop_run(&mut self) {
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
        match self.config.save() {
            Ok(()) => tracing::info!("Config autosaved to {}", self.config.path.display()),
            Err(e) => tracing::error!("Could not autosave Config: {e}"),
        }
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
        if self.graphs.should_update()
            && self.embassy.is_connected()
            && self.status.is_system_running()
        {
            self.graphs
                .update(self.status.get_surveyor_status_response());
        }
        render_config_panel(self, ctx);
        render_graph_panel(self, ctx);
        render_ecc_panel(self, ctx);
        render_data_router_panel(self, ctx);
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}
//*************//
//  APP IMPL  //
//*************//
