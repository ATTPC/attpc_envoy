//! # Extending the system
//! There are some aspects of the GET DAQ system that result in hard edges when running
//! an experiment. This scripting command system aims to eliminate this flaws by allowing
//! users to extend the functionality of the DAQ.This is done by writting a shell script in the
//! `scripts` directory and then tying the scrip to to a function here, which then can be called using
//! the `execute` function with the appropriate CommandName.
//!
//! ## Examples
//! By default, there are extensions for backing up the ECC configuration files, moving the .graw
//! files to an experiment specific directory with run subdirectories, and checking if directories exist on the ECC machines.
use super::constants::{BACKUP_CONFIG_DIR, CONFIG_DIR, SCRIPT_DIR};
use crate::envoy::surveyor_envoy::SurveyorResponse;
use std::process::Command;

/// The status of a command which was executed
#[derive(Debug, Clone)]
pub enum CommandStatus {
    Success,
    Failure,
    CouldNotExecute,
}

/// CommandNames are tied to one of the functions which is callable by the execute function in
/// this module. All commands must have the same function signature. This allows for relatively straightforward
/// command sending from the UI. Typically these commands wrap the std::process::Command object which is used to
/// run a shell script on a remote machine. Think of this like a *really* primitive scripting engine.
#[derive(Debug, Clone)]
pub enum CommandName {
    MoveGrawFiles,
    BackupConfig,
    CheckRunExists,
}

impl std::fmt::Display for CommandName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MoveGrawFiles => write!(f, "MoveGrawFiles"),
            Self::BackupConfig => write!(f, "BackupConfig"),
            Self::CheckRunExists => write!(f, "CheckRunExists"),
        }
    }
}

impl CommandName {
    /// Retrieve the function associated with this CommandName
    pub fn get_function(
        &self,
    ) -> impl Fn(&[SurveyorResponse], &str, &i32) -> Result<CommandStatus, std::io::Error> {
        match self {
            Self::MoveGrawFiles => move_graw_files,
            Self::BackupConfig => backup_config,
            Self::CheckRunExists => check_run_exists,
        }
    }
}

/// This is the function used by the rest of the crate. Pass in a CommandName with the required data and recieve a command status
/// based on the behavior of the command.
pub fn execute(
    command: CommandName,
    surveyor_data: &[SurveyorResponse],
    experiment: &str,
    run_number: &i32,
) -> CommandStatus {
    match command.get_function()(surveyor_data, experiment, run_number) {
        Ok(stat) => return stat,
        Err(e) => {
            tracing::error!("Could not execute command {}: {}", command, e);
            return CommandStatus::CouldNotExecute;
        }
    }
}

/// Move the graw data files after a run is stopped
pub fn move_graw_files(
    surveyor_data: &[SurveyorResponse],
    experiment: &str,
    run_number: &i32,
) -> Result<CommandStatus, std::io::Error> {
    let sub_command = format!("{SCRIPT_DIR}move_graw.sh");
    let mut ret_stat = CommandStatus::Success;
    for data in surveyor_data {
        let output = Command::new("zsh")
            .args([
                &sub_command,
                &data.address,
                &data.location,
                experiment,
                &(run_number.to_string()),
            ])
            .output()?;
        if !output.status.success() {
            ret_stat = CommandStatus::Failure;
        }
    }
    Ok(ret_stat)
}

/// Back up the ECC configuration files after a run is stopped
pub fn backup_config(
    _: &[SurveyorResponse],
    experiment: &str,
    run_number: &i32,
) -> Result<CommandStatus, std::io::Error> {
    let sub_command = format!("{SCRIPT_DIR}backup_configs.sh");
    let output = Command::new("zsh")
        .args([
            &sub_command,
            CONFIG_DIR,
            BACKUP_CONFIG_DIR,
            experiment,
            &(run_number.to_string()),
        ])
        .output()?;
    if output.status.success() {
        return Ok(CommandStatus::Success);
    } else {
        return Ok(CommandStatus::Failure);
    }
}

/// Check to see if a run number was already used before starting a run
pub fn check_run_exists(
    surveyor_data: &[SurveyorResponse],
    experiment: &str,
    run_number: &i32,
) -> Result<CommandStatus, std::io::Error> {
    let sub_command = format!("{SCRIPT_DIR}test_graw.sh");
    let output = Command::new("zsh")
        .args([
            &sub_command,
            &surveyor_data[0].address,
            &surveyor_data[0].location,
            experiment,
            &(run_number.to_string()),
        ])
        .output()?;

    if output.status.success() {
        return Ok(CommandStatus::Success);
    } else {
        return Ok(CommandStatus::Failure);
    }
}
