use super::ecc_envoy::startup_ecc_envoys;
use super::error::EmbassyError;
use super::message::{EmbassyMessage, MessageKind};
use super::surveyor_envoy::startup_surveyor_envoys;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

/// The embassy is the bridge between the async envoys and
/// the synchronous UI-application. The embassy is essentially a
/// container of channels used to communicate back-and-forth between these
/// two runtimes.
#[derive(Debug)]
pub struct Embassy {
    ecc_senders: HashMap<usize, mpsc::Sender<EmbassyMessage>>,
    envoy_reciever: mpsc::Receiver<EmbassyMessage>,
    cancel: broadcast::Sender<EmbassyMessage>,
}

impl Embassy {
    /// Create an Embassy with some communication channels
    pub fn new(
        envoy_reciever: mpsc::Receiver<EmbassyMessage>,
        ecc_senders: HashMap<usize, mpsc::Sender<EmbassyMessage>>,
        cancel: broadcast::Sender<EmbassyMessage>,
    ) -> Self {
        Embassy {
            ecc_senders,
            envoy_reciever,
            cancel,
        }
    }

    /// Shutdown the Embassy and cancel any tasks
    pub fn shutdown(&mut self) {
        let cancel_message = EmbassyMessage::compose_cancel();
        self.cancel
            .send(cancel_message)
            .expect("Some how all the envoys are already dead?");
    }

    /// Submit an EmbassyMessage. Currently only communicates with ECCEnvoys.
    pub fn submit_message(&mut self, message: EmbassyMessage) -> Result<(), EmbassyError> {
        if message.kind == MessageKind::ECCOperation {
            if let Some(sender) = self.ecc_senders.get_mut(&message.id) {
                sender.blocking_send(message)?;
            }
        }
        Ok(())
    }

    /// Poll the Embassy to see if any messages were recieved from the envoys
    pub fn poll_messages(&mut self) -> Result<Vec<EmbassyMessage>, EmbassyError> {
        let mut messages: Vec<EmbassyMessage> = vec![];
        loop {
            match self.envoy_reciever.try_recv() {
                Ok(message) => messages.push(message.into()),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(EmbassyError::MessageRecieveError)
                }
            };
        }
        Ok(messages)
    }
}

/// This is the function to create and connect an Embassy as well as all of the envoys given a tokio runtime and experiment name.
pub fn connect_embassy(
    runtime: &mut tokio::runtime::Runtime,
    experiment: &str,
) -> (Embassy, Vec<tokio::task::JoinHandle<()>>) {
    let (envoy_tx, embassy_rx) = mpsc::channel::<EmbassyMessage>(33);
    let (cancel_tx, _) = broadcast::channel::<EmbassyMessage>(10);

    let (mut handles, ecc_switchboard) =
        startup_ecc_envoys(runtime, experiment, &envoy_tx, &cancel_tx);
    let mut sur_handles = startup_surveyor_envoys(runtime, &envoy_tx, &cancel_tx);

    let embassy = Embassy::new(embassy_rx, ecc_switchboard, cancel_tx);

    handles.append(&mut sur_handles);
    return (embassy, handles);
}
