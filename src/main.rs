//! # attpc_envoy
//!
//! attpc_envoy is a Data Acquisition Hub for the Active Target Time Projection Chamber
//! written in Rust. It provides an async framework for querying the various DAQ elements
//! of the AT-TPC with a clean user interface. The primary goal is to provide an error-safe
//! system from which to run and manage the data aqcuistion, without sacrificing
//! performance.
//!
//! ## Download and Installation
//!
//! To build attpc_envoy you need a rust compiler which can be installed using instructions
//! from the Rust [website](https://www.rust-lang.org).
//!
//! To download attpc_envoy use the command:
//!
//! ```bash
//! git clone https://github.com/gwm17/attpc_envoy.git
//! ```
//!
//! To build the project and run it, enter the repository and use the following command
//!
//! ```bash
//! cargo run -r
//! ```
//!
//! attpc_envoy aims to be cross platform and supports Mac, Windows, and Linux. However,
//! since the AT-TPC group primarily uses MacOS for DAQ machines, attpc_envoy is only
//! guaranteed to run on MacOS. Other platforms are supported as best-effort only.
//!
//! ## What is attpc_envoy exactly?
//!
//! attpc_envoy is essentially a control panel that communicates to the distributed DAQ
//! services used to run the AT-TPC. There are a few key assumptions made by the envoy
//! system that you should be aware of:
//!
//! - attpc_envoy assumes that you are using the AT-TPC network infrastructure, including
//! the expected IP address assignments for DAQ machines.
//! - Each DAQ machine is expected to be running three programs: `getECCServer`,
//! `dataRouter` or `DataExporter`, and `attpc_sentry`
//! - There are the typical number of DAQ machines (at time of writting, 11)
//! - GETDAQ configuration files are located in the expected AT-TPC directory structure
//!
//! If you do not meet these requirements, or are unsure, you can checkout the
//! [constants](https://attpc.github.io/attpc_envoy/attpc_envoy/envoy/constants/index.html)
//! docs to view some of them.
//!
//! ## Configuration
//!
//! It is not recommended to create a configuration by hand. Instead, it is best to use the
//! UI to set configuration parameters and then use the File->Save, and File->Load menus to
//! handle configuration saving and loading. However, we will outline the format here for
//! clarity.
//!
//! attpc_envoy uses YAML to define its configuration. Below is an example configuration
//! file:
//!
//! ```yaml
//! experiment: Exp
//! run_number: 0
//! description: Write here
//! fields:
//!   B-Field (T): ''
//!   Beam: ''
//!   E-Drift (V): ''
//!   E-Trans (V): ''
//!   Energy (MeV/U): ''
//!   GET Freq. (MHz): ''
//!   Pressure (Torr): ''
//!   Target Gas: ''
//!   V_Cathode (kV): ''
//!   V_MM (V): ''
//!   V_THGEM (V): ''
//! ```
//!
//! `experiment` is the experiment identifier, and should match the identifier used for the
//! GET configuration files. `run_number` is the current run number. `description` is a
//! brief discription of the run. `fields` is an extensible list of run logged info. All
//! `fields` values are stored as strings even if they are more naturally a numeric type to
//! avoid boxing.

mod envoy;
mod ui;

use std::path::PathBuf;
use tokio::runtime::Builder;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use ui::app::EnvoyApp;

/// Program entry point
fn main() {
    //Create the async runtime
    let runtime: tokio::runtime::Runtime = Builder::new_multi_thread()
        .worker_threads(5)
        .enable_time()
        .enable_io()
        .build()
        .expect("Could not startup async runtime!");

    let log_path = PathBuf::from("logs/");
    if !log_path.exists() {
        match std::fs::create_dir(&log_path) {
            Ok(_) => (),
            Err(e) => {
                println!("Could not make directory for logs! Error: {e}");
                return;
            }
        }
    }
    let rolling_log = tracing_appender::rolling::daily(log_path, "attpc_envoy_log");
    let stderr = std::io::stderr.with_max_level(tracing::Level::ERROR);

    //Create our logging/tracing system.
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_writer(stderr.and(rolling_log))
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Could not initialize the tracing system!");

    tracing::info!("Tracing initialized!");

    //Start our application
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("AT-TPC Envoy")
            .with_inner_size(eframe::epaint::vec2(1600.0, 1060.0)),
        ..Default::default()
    };
    match eframe::run_native(
        "ATTPC Envoy",
        native_options,
        Box::new(|cc| Ok(Box::new(EnvoyApp::new(cc, runtime)))),
    ) {
        Ok(()) => (),
        Err(e) => tracing::error!("Eframe error: {}", e),
    }
}
