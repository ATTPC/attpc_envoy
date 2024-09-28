use crate::envoy::constants::{MUTANT_ID, NUMBER_OF_MODULES};
use crate::envoy::ecc_envoy::{ECCOperationResponse, ECCStatusResponse};
use crate::envoy::ecc_operation::ECCStatus;
use crate::envoy::error::EmbassyError;
use crate::envoy::message::{EmbassyMessage, MessageKind};
use crate::envoy::surveyor_envoy::SurveyorResponse;
use crate::envoy::surveyor_status::SurveyorStatus;

/// Structure used to manage the status of all of the envoys. We need a centralized location
/// because we also want to express the status of the entire system, not just the individuals.
/// It has observer-like behavior where it reads a list of messages from the embassy and handles
/// the information appropriately.
#[derive(Debug)]
pub struct StatusManager {
    ecc_status: Vec<ECCStatusResponse>,
    surveyor_status: Vec<SurveyorResponse>,
    ecc_holds: Vec<bool>,
}

impl StatusManager {
    /// Create a new manager with space for the statuses of all envoys
    pub fn new() -> Self {
        let eccs = vec![ECCStatusResponse::default(); NUMBER_OF_MODULES];
        let surs = vec![SurveyorResponse::default(); NUMBER_OF_MODULES - 1];
        let holds = vec![false; NUMBER_OF_MODULES];
        return Self {
            ecc_status: eccs,
            surveyor_status: surs,
            ecc_holds: holds,
        };
    }

    /// Reset the data of all the envoys
    pub fn reset(&mut self) {
        for eccs in self.ecc_status.iter_mut() {
            *eccs = ECCStatusResponse::default();
        }

        for surs in self.surveyor_status.iter_mut() {
            *surs = SurveyorResponse::default();
        }
    }

    /// Read messages from the embassy and look for ECC or Surveyor status respsonses.
    /// Set the status of the given module to match the message.
    pub fn handle_messages(&mut self, messages: &[EmbassyMessage]) -> Result<(), EmbassyError> {
        for message in messages {
            let module_id = message.id;
            match message.kind {
                MessageKind::ECCOperation => {
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
                    self.ecc_holds[module_id as usize] = false;
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

                    if self.ecc_holds[module_id as usize] == false {
                        self.ecc_status[module_id as usize] = resp;
                    }
                }
                MessageKind::Surveyor => {
                    let resp: SurveyorResponse = message.try_into()?;
                    self.surveyor_status[module_id as usize] = resp;
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
        return ECCStatus::from(sys_status);
    }

    /// Is the entire system at the ECC Ready status
    pub fn is_system_ready(&self) -> bool {
        let sys_stat = self.get_system_ecc_status();
        match sys_stat {
            ECCStatus::Ready => true,
            _ => false,
        }
    }

    /// Is the system in an active run state
    pub fn is_system_running(&self) -> bool {
        let sys_stat = self.get_system_ecc_status();
        match sys_stat {
            ECCStatus::Running => true,
            _ => false,
        }
    }

    /// Are all of the CoBos running, waiting for the MuTaNT
    pub fn is_all_but_mutant_running(&self) -> bool {
        let sys_status = self.ecc_status[0].state;
        for status in self.ecc_status[..((NUMBER_OF_MODULES - 1) as usize)].iter() {
            if sys_status != status.state {
                return false;
            }
        }

        match ECCStatus::from(sys_status) {
            ECCStatus::Running => return true,
            _ => return false,
        }
    }

    /// Is everyone but the MuTaNT at the Ready status
    pub fn is_all_but_mutant_ready(&self) -> bool {
        let sys_status = self.ecc_status[0].state;
        for status in self.ecc_status[..((NUMBER_OF_MODULES - 1) as usize)].iter() {
            if sys_status != status.state {
                return false;
            }
        }

        match ECCStatus::from(sys_status) {
            ECCStatus::Ready => return true,
            _ => return false,
        }
    }

    /// Is the MuTaNT stopped (not running)
    pub fn is_mutant_stopped(&self) -> bool {
        match self.get_ecc_status(MUTANT_ID as usize) {
            ECCStatus::Running => return false,
            _ => return true,
        }
    }

    /// Is the MuTaNT at the Prepared status
    pub fn is_mutant_prepared(&self) -> bool {
        match self.get_ecc_status(MUTANT_ID as usize) {
            ECCStatus::Prepared => return true,
            _ => return false,
        }
    }

    /// Is the MuTaNT at the Ready status
    pub fn is_mutant_ready(&self) -> bool {
        match self.get_ecc_status(MUTANT_ID as usize) {
            ECCStatus::Ready => return true,
            _ => return false,
        }
    }

    /// Returns a slice of all SurveyorResponses (SurveyorEnvoy statuses)
    pub fn get_surveyor_status_response(&self) -> &[SurveyorResponse] {
        &self.surveyor_status
    }

    /// Get the status of a specific ECCEnvoy
    pub fn get_ecc_status(&self, id: usize) -> ECCStatus {
        return ECCStatus::from(self.ecc_status[id].state);
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
            match self.get_ecc_status(MUTANT_ID) {
                ECCStatus::Prepared => return true,
                ECCStatus::Ready => return true,
                _ => return false,
            }
        } else if status == ECCStatus::Prepared && id == MUTANT_ID {
            return self.is_all_but_mutant_ready();
        } else {
            return status.can_go_forward();
        }
    }

    /// Retrieve the Surveyor/DataRouter system status. System status matches the envoy status if all
    /// envoys have the same status. If not, the system status is Inconsistent.
    pub fn get_surveyor_system_status(&self) -> SurveyorStatus {
        let sys_status = self.surveyor_status[0].state;
        for status in self.surveyor_status.iter() {
            if sys_status != status.state {
                return SurveyorStatus::Inconsistent;
            }
        }
        return SurveyorStatus::from(sys_status);
    }

    /// Get the status of a specific SurveyorEnvoy
    #[allow(dead_code)]
    pub fn get_surveyor_status(&self, id: usize) -> SurveyorStatus {
        return SurveyorStatus::from(self.surveyor_status[id].state);
    }
}
