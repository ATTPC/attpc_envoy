use super::constants::{BACK_CONFIG_DIR, CONFIG_DIR, MUTANT_ID, NUMBER_OF_MODULES};
use super::ecc_operation::ECCOperation;
use super::embassy::Embassy;
use super::error::EmbassyError;
use super::message::EmbassyMessage;
use super::sentry_types::{SentryOperation, SentryParameters};
use super::status_manager::StatusManager;
use std::fs::read_dir;
use std::path::PathBuf;

pub fn poll_embassy(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
) -> Result<(), EmbassyError> {
    if !embassy.is_connected() {
        return Ok(());
    }
    match embassy.poll_messages() {
        Ok(messages) => {
            status_manager.handle_messages(&messages)?;
        }
        Err(e) => tracing::error!("Embassy ran into an error polling the envoys: {}", e),
    };
    Ok(())
}

/// Send a transition command to some of the ECC operation envoys. Transitions are either forward or backward
/// depending on the is_forward flag. What type of transition is determined by the current state of the envoy as last recorded
/// by the status envoy.
pub fn transition_ecc(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
    ids: Vec<usize>,
    is_forward: bool,
) {
    if ids.is_empty() {
        return;
    }
    if !embassy.is_connected() {
        tracing::error!("Some how trying to operate on ECC whilst disconnected!");
        return;
    }
    for id in ids {
        let status = status_manager.get_ecc_status(id);
        let operation: ECCOperation = if is_forward {
            status.get_forward_operation()
        } else {
            status.get_backward_operation()
        };
        match operation {
            ECCOperation::Invalid => (),
            _ => match embassy.submit_message(EmbassyMessage::compose(operation, id)) {
                Ok(()) => (),
                Err(e) => tracing::error!("Embassy had an error sending a message: {}", e),
            },
        }
        status_manager.set_ecc_busy(id);
    }
}

/// Send the mutant forward from described to prepared and block on waiting
/// until that transition is complete
pub fn forward_mutant_prepared_blocking(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
) -> Result<(), EmbassyError> {
    transition_ecc(embassy, status_manager, vec![MUTANT_ID], true);
    loop {
        poll_embassy(embassy, status_manager)?;
        if status_manager.is_mutant_prepared() {
            break;
        }
    }
    Ok(())
}

/// Send all of the CoBos forward from prepared to Ready (Configure transition) and
/// block on waiting until all of those transitions are complete
pub fn forward_cobos_ready_blocking(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
) -> Result<(), EmbassyError> {
    let all_ids_but_mutant: Vec<usize> = (0..(NUMBER_OF_MODULES - 1)).collect();
    transition_ecc(embassy, status_manager, all_ids_but_mutant, true);
    loop {
        poll_embassy(embassy, status_manager)?;
        if status_manager.is_all_but_mutant_ready() {
            break;
        }
    }
    Ok(())
}

/// Transition all of the envoys forward (Progress)
/// This is slightly more complicated as order matters for two of the phases (Prepare and Configure)
pub fn forward_transition_all(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
) -> Result<(), EmbassyError> {
    let system = status_manager.get_system_ecc_status();
    let all_ids_but_mutant: Vec<usize> = (0..(NUMBER_OF_MODULES - 1)).collect();
    let ids: Vec<usize> = (0..NUMBER_OF_MODULES).collect();
    match system.get_forward_operation() {
        //Describe operation: order doesn't matter
        ECCOperation::Describe => {
            transition_ecc(embassy, status_manager, ids, true);
            Ok(())
        }
        //Prepare operation: mutant first, then cobos
        ECCOperation::Prepare => {
            forward_mutant_prepared_blocking(embassy, status_manager)?;
            transition_ecc(embassy, status_manager, all_ids_but_mutant, true);
            Ok(())
        }
        //Configure operation: cobos first, then mutant
        ECCOperation::Configure => {
            forward_cobos_ready_blocking(embassy, status_manager)?;
            transition_ecc(embassy, status_manager, vec![MUTANT_ID], true);
            Ok(())
        }
        e => Err(EmbassyError::InvalidTransition(e)),
    }
}

/// Transition all of the envoys backwards (Regresss)
pub fn backward_transition_all(embassy: &mut Embassy, status_manager: &mut StatusManager) {
    let ids: Vec<usize> = (0..(NUMBER_OF_MODULES)).collect();
    transition_ecc(embassy, status_manager, ids, false);
}

/// Start the MuTaNT
pub fn start_mutant(embassy: &mut Embassy) -> Result<(), EmbassyError> {
    embassy.submit_message(EmbassyMessage::compose(ECCOperation::Start, MUTANT_ID))
}

/// Reconfigure the MuTaNT (Regress once, and then Configure again) to
/// restart the event numbers and timestamps. This is used when starting
/// a new run.
pub fn reconfigure_mutant_blocking(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
) -> Result<(), EmbassyError> {
    let mutant = vec![MUTANT_ID];
    transition_ecc(embassy, status_manager, mutant.clone(), false);
    loop {
        poll_embassy(embassy, status_manager)?;
        if status_manager.is_mutant_prepared() {
            break;
        }
    }
    transition_ecc(embassy, status_manager, mutant, true);
    loop {
        poll_embassy(embassy, status_manager)?;
        if status_manager.is_mutant_ready() {
            break;
        }
    }
    Ok(())
}

/// Stop the MuTaNT and wait until that is completed
pub fn stop_mutant_blocking(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
) -> Result<(), EmbassyError> {
    embassy.submit_message(EmbassyMessage::compose(ECCOperation::Stop, MUTANT_ID))?;

    //Wait for mutant to stop
    loop {
        poll_embassy(embassy, status_manager)?;
        if status_manager.is_mutant_stopped() {
            break;
        }
    }

    Ok(())
}

/// Start all of the CoBos and wait until that is completed
pub fn start_cobos_blocking(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
) -> Result<(), EmbassyError> {
    for id in 0..(NUMBER_OF_MODULES - 1) {
        embassy.submit_message(EmbassyMessage::compose(ECCOperation::Start, id))?;
    }

    //Wait for good CoBo status
    loop {
        poll_embassy(embassy, status_manager)?;
        if status_manager.is_all_but_mutant_running() {
            break;
        }
    }
    Ok(())
}

/// Stop all of the CoBos
pub fn stop_cobos(embassy: &mut Embassy) -> Result<(), EmbassyError> {
    for id in 0..(NUMBER_OF_MODULES - 1) {
        embassy.submit_message(EmbassyMessage::compose(ECCOperation::Stop, id))?;
    }
    Ok(())
}

pub fn backup_configs(experiment: &str, run_number: &i32) -> Result<(), EmbassyError> {
    let config_path = PathBuf::from(CONFIG_DIR);
    let cobo_path = config_path.join("describe-cobo");
    let bck_config_path =
        PathBuf::from(BACK_CONFIG_DIR).join(format!("{}/run_{:04}", experiment, run_number));
    let bck_cobo_path = bck_config_path.join("describe-cobo");

    if !bck_config_path.exists() {
        std::fs::create_dir_all(&bck_cobo_path).expect("Failed creating cobo backup directories");
    }

    let prep_name = format!("prepare-{}.xcfg", experiment);
    let desc_name = format!("describe-{}.xcfg", experiment);
    let conf_name = format!("configure-{}.xcfg", experiment);

    std::fs::copy(
        config_path.join(&prep_name),
        bck_config_path.join(&prep_name),
    )
    .expect("Could not back up prepare file");
    std::fs::copy(
        config_path.join(&desc_name),
        bck_config_path.join(&desc_name),
    )
    .expect("Could not back up desc file");
    std::fs::copy(
        config_path.join(&conf_name),
        bck_config_path.join(&conf_name),
    )
    .expect("Could not back up conf file");

    let reader = read_dir(cobo_path).expect("Could not read cobo dir");
    for maybe_entry in reader {
        match maybe_entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    std::fs::copy(
                        &path,
                        bck_cobo_path
                            .join(path.file_name().expect("Cobo file doesn't have a name?")),
                    )
                    .expect("Could not backup cobo dir file");
                }
            }
            Err(e) => tracing::error!("Could not get an entry in the cobo dir! {e}"),
        }
    }
    Ok(())
}

pub fn catalog_run(
    embassy: &mut Embassy,
    status_manager: &mut StatusManager,
    experiment: &str,
    run_number: &i32,
) -> Result<(), EmbassyError> {
    embassy.submit_message(EmbassyMessage::compose(
        SentryOperation::Catalog(SentryParameters {
            experiment: String::from(experiment),
            run_number: *run_number,
        }),
        0,
    ))?;

    loop {
        poll_embassy(embassy, status_manager)?;
        if status_manager.has_sentry_cataloged() {
            break;
        }
    }

    Ok(())
}
