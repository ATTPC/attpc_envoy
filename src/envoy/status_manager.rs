use super::constants::{MUTANT_ID, NUMBER_OF_MODULES};
use super::ecc_envoy::{ECCOperationResponse, ECCStatusResponse};
use super::ecc_operation::ECCStatus;
use super::error::EmbassyError;
use super::message::{EmbassyMessage, MessageKind};
use super::sentry_types::{SentryServerStatus, SentryStatus};

/// Structure used to manage the status of all of the envoys. We need a centralized location
/// because we also want to express the status of the entire system, not just the individuals.
/// It has observer-like behavior where it reads a list of messages from the embassy and handles
/// the information appropriately.
#[derive(Debug)]
pub struct StatusManager {
    ecc_status: Vec<ECCStatusResponse>,
    sentry_status: Vec<SentryStatus>,
    ecc_holds: Vec<bool>,
}

impl StatusManager {
    /// Create a new manager with space for the statuses of all envoys
    pub fn new() -> Self {
        let eccs = vec![ECCStatusResponse::default(); NUMBER_OF_MODULES];
        let sentries = vec![SentryStatus::default(); NUMBER_OF_MODULES - 1];
        let holds = vec![false; NUMBER_OF_MODULES];
        Self {
            ecc_status: eccs,
            sentry_status: sentries,
            ecc_holds: holds,
        }
    }

    /// Reset the data of all the envoys
    pub fn reset(&mut self) {
        for eccs in self.ecc_status.iter_mut() {
            *eccs = ECCStatusResponse::default();
        }

        for surs in self.sentry_status.iter_mut() {
            *surs = SentryStatus::default();
        }
    }

    /// Read messages from the embassy and look for ECC or Surveyor status respsonses.
    /// Set the status of the given module to match the message.
    pub fn handle_messages(&mut self, messages: &[EmbassyMessage]) -> Result<(), EmbassyError> {
        for message in messages {
            let module_id = message.id;
            match message.kind {
                MessageKind::ECCOpResponse => {
                    let resp: ECCOperationResponse = message.try_into()?;
                    if resp.error_code != 0 {
                        tracing::error!(
                            "ECC Operation failed with error code {} for module id {}: {}",
                            resp.error_code,
                            module_id,
                            resp.error_message
                        );
                    } else {
                        tracing::info!("ECC Operation completed for module id {}", module_id);
                    }
                    self.ecc_holds[module_id] = false;
                }
                MessageKind::ECCStatus => {
                    let resp: ECCStatusResponse = message.try_into()?;
                    if resp.error_code != 0 {
                        tracing::error!(
                            "ECC Status failed with error code {} for module id {}: {}",
                            resp.error_code,
                            module_id,
                            resp.error_message
                        )
                    }

                    if !self.ecc_holds[module_id] {
                        self.ecc_status[module_id] = resp;
                    }
                }
                MessageKind::SentryStatus => {
                    let resp: SentryStatus = message.try_into()?;
                    self.sentry_status[module_id] = resp;
                }
                _ => {
                    tracing::warn!("Some how recieved a message of kind {} which is not a valid recieving kind!", message.kind);
                }
            }
        }
        Ok(())
    }

    /// Retrieve a slice for all of the ECCStatusResponses (the status of all of the ECCEnvoys)
    pub fn get_ecc_status_response(&self) -> &[ECCStatusResponse] {
        &self.ecc_status
    }

    /// Retrieve the system ECC status. System status matches the envoy status if all
    /// envoys have the same status. If not, the system status is Inconsistent.
    pub fn get_system_ecc_status(&self) -> ECCStatus {
        let sys_status = self.ecc_status[0].state;
        for status in self.ecc_status.iter() {
            if sys_status != status.state {
                return ECCStatus::Inconsistent;
            }
        }
        ECCStatus::from(sys_status)
    }

    /// Is the entire system at the ECC Ready status
    pub fn is_system_ready(&self) -> bool {
        let sys_stat = self.get_system_ecc_status();
        matches!(sys_stat, ECCStatus::Ready)
    }

    /// Is the system in an active run state
    pub fn is_system_running(&self) -> bool {
        matches!(self.get_system_ecc_status(), ECCStatus::Running)
    }

    /// Are all of the CoBos running, waiting for the MuTaNT
    pub fn is_all_but_mutant_running(&self) -> bool {
        let sys_status = self.ecc_status[0].state;
        for status in self.ecc_status[..(NUMBER_OF_MODULES - 1)].iter() {
            if sys_status != status.state {
                return false;
            }
        }

        matches!(ECCStatus::from(sys_status), ECCStatus::Running)
    }

    /// Is everyone but the MuTaNT at the Ready status
    pub fn is_all_but_mutant_ready(&self) -> bool {
        let sys_status = self.ecc_status[0].state;
        for status in self.ecc_status[..(NUMBER_OF_MODULES - 1)].iter() {
            if sys_status != status.state {
                return false;
            }
        }

        matches!(ECCStatus::from(sys_status), ECCStatus::Ready)
    }

    /// Is the MuTaNT stopped (not running)
    pub fn is_mutant_stopped(&self) -> bool {
        matches!(self.get_ecc_status(MUTANT_ID), ECCStatus::Running)
    }

    /// Is the MuTaNT at the Prepared status
    pub fn is_mutant_prepared(&self) -> bool {
        matches!(self.get_ecc_status(MUTANT_ID), ECCStatus::Prepared)
    }

    /// Is the MuTaNT at the Ready status
    pub fn is_mutant_ready(&self) -> bool {
        matches!(self.get_ecc_status(MUTANT_ID), ECCStatus::Ready)
    }

    /// Returns a slice of all SentryStatuss (SurveyorEnvoy statuses)
    pub fn get_sentry_status_response(&self) -> &[SentryStatus] {
        &self.sentry_status
    }

    /// Get the status of a specific ECCEnvoy
    pub fn get_ecc_status(&self, id: usize) -> ECCStatus {
        ECCStatus::from(self.ecc_status[id].state)
    }

    /// Set a specific ECCEnvoy as Busy
    pub fn set_ecc_busy(&mut self, id: usize) {
        if id > MUTANT_ID {
            return;
        }

        self.ecc_status[id].state = ECCStatus::Busy.into();
        self.ecc_holds[id] = true;
    }

    /// Check if an ECCEnvoy can go forward (progress)
    pub fn can_ecc_go_forward(&self, id: usize) -> bool {
        let status = self.get_ecc_status(id);
        if status == ECCStatus::Described && id != MUTANT_ID {
            matches!(
                self.get_ecc_status(MUTANT_ID),
                ECCStatus::Prepared | ECCStatus::Ready
            )
        } else if status == ECCStatus::Prepared && id == MUTANT_ID {
            self.is_all_but_mutant_ready()
        } else {
            status.can_go_forward()
        }
    }

    pub fn has_sentry_cataloged(&self) -> bool {
        for stat in self.sentry_status.iter() {
            if stat.data_path_files != 0 {
                return false;
            }
        }
        true
    }

    /// Retrieve the Surveyor/DataRouter system status. System status matches the envoy status if all
    /// envoys have the same status. If not, the system status is Inconsistent.
    pub fn get_sentry_server_system_status(&self) -> SentryServerStatus {
        let sys_status = SentryServerStatus::from(&self.sentry_status[0]);
        for status in self.sentry_status.iter() {
            let this_status = SentryServerStatus::from(status);
            if sys_status != this_status {
                return SentryServerStatus::Inconsistent;
            }
        }
        sys_status
    }

    /// Get the status of a specific SurveyorEnvoy
    #[allow(dead_code)]
    pub fn get_sentry_server_status(&self, id: usize) -> SentryServerStatus {
        SentryServerStatus::from(&self.sentry_status[id])
    }
}
