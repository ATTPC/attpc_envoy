use super::ecc_envoy::startup_ecc_envoys;
use super::error::EmbassyError;
use super::message::{EmbassyMessage, MessageKind};
use super::sentry_envoy::startup_sentry_envoys;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// The embassy is the bridge between the async envoys and
/// the synchronous UI-application. The embassy is essentially a
/// container of channels used to communicate back-and-forth between these
/// two runtimes.
#[derive(Debug)]
pub struct Embassy {
    ecc_senders: HashMap<usize, mpsc::Sender<EmbassyMessage>>,
    sentry_sender: Option<broadcast::Sender<EmbassyMessage>>,
    envoy_reciever: Option<mpsc::Receiver<EmbassyMessage>>,
    cancel: Option<broadcast::Sender<EmbassyMessage>>,
    handles: Option<Vec<JoinHandle<()>>>,
    runtime: Runtime,
    is_connected: bool,
}

impl Embassy {
    /// Create an Embassy with a tokio Runtime
    pub fn new(rt: Runtime) -> Self {
        Embassy {
            ecc_senders: HashMap::new(),
            sentry_sender: None,
            envoy_reciever: None,
            cancel: None,
            handles: None,
            runtime: rt,
            is_connected: false,
        }
    }

    /// Start the embassy service, connecting it to the various envoys
    pub fn startup(&mut self, experiment: &str) {
        let (envoy_tx, embassy_rx) = mpsc::channel::<EmbassyMessage>(33);
        let (cancel_tx, _) = broadcast::channel::<EmbassyMessage>(10);
        let (sentry_tx, _) = broadcast::channel::<EmbassyMessage>(10);

        let (mut handles, ecc_switchboard) =
            startup_ecc_envoys(&mut self.runtime, experiment, &envoy_tx, &cancel_tx);
        let mut sentry_handles =
            startup_sentry_envoys(&mut self.runtime, &envoy_tx, &sentry_tx, &cancel_tx);
        handles.append(&mut sentry_handles);
        self.ecc_senders = ecc_switchboard;
        self.sentry_sender = Some(sentry_tx);
        self.envoy_reciever = Some(embassy_rx);
        self.cancel = Some(cancel_tx);
        self.is_connected = true;
        self.handles = Some(handles);
    }

    /// Shutdown the Embassy and cancel any tasks
    pub fn shutdown(&mut self) -> Result<(), EmbassyError> {
        let cancel_message = EmbassyMessage::compose_cancel();
        if let Some(tx) = &self.cancel {
            tx.send(cancel_message)
                .expect("Some how all of the envoys were already dead!");
        }
        if let Some(handles) = self.handles.take() {
            for handle in handles {
                self.runtime.block_on(handle)?
            }
        }
        self.is_connected = false;
        Ok(())
    }

    /// Submit an EmbassyMessage. Currently only communicates with ECCEnvoys.
    pub fn submit_message(&mut self, message: EmbassyMessage) -> Result<(), EmbassyError> {
        match message.kind {
            MessageKind::ECCOperation => {
                if let Some(sender) = self.ecc_senders.get_mut(&message.id) {
                    sender.blocking_send(message)?;
                }
            }
            MessageKind::SentryOperation => {
                if let Some(sender) = &self.sentry_sender {
                    sender.send(message)?;
                }
            }
            _ => (),
        }

        Ok(())
    }

    /// Poll the Embassy to see if any messages were recieved from the envoys
    pub fn poll_messages(&mut self) -> Result<Vec<EmbassyMessage>, EmbassyError> {
        let mut messages: Vec<EmbassyMessage> = vec![];
        if let Some(rx) = &mut self.envoy_reciever {
            loop {
                match rx.try_recv() {
                    Ok(message) => messages.push(message),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        return Err(EmbassyError::FailedRecieve)
                    }
                }
            }
        }
        Ok(messages)
    }

    /// Is the embassy connected to the envoys
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// How many tasks have been spawned
    pub fn number_of_tasks(&self) -> usize {
        if let Some(handles) = &self.handles {
            handles.len()
        } else {
            0
        }
    }
}
