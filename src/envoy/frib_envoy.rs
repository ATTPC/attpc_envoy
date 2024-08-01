use std::time::Duration;

use super::error::EnvoyError;
use super::frib_operation::FribStatus;
use super::{constants::FRIBDAQ_ADDRESS, message::EmbassyMessage};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct FribEnvoy {
    control_address: String,
    response_address: String,
    incoming: mpsc::Receiver<EmbassyMessage>,
    outgoing: mpsc::Sender<EmbassyMessage>,
    cancel: broadcast::Receiver<EmbassyMessage>,
}

impl FribEnvoy {
    pub fn new(
        control_port: i32,
        response_port: i32,
        rx: mpsc::Receiver<EmbassyMessage>,
        tx: mpsc::Sender<EmbassyMessage>,
        cancel: broadcast::Receiver<EmbassyMessage>,
    ) -> Result<Self, EnvoyError> {
        let control_address = format!("{FRIBDAQ_ADDRESS}:{control_port}");
        let response_address = format!("{FRIBDAQ_ADDRESS}:{response_port}");

        Ok(Self {
            control_address,
            response_address,
            incoming: rx,
            outgoing: tx,
            cancel,
        })
    }

    pub async fn wait_for_operation(&mut self) -> Result<(), EnvoyError> {
        let timeout = Duration::from_secs(120);
        let mut control_stream =
            match tokio::time::timeout(timeout, TcpStream::connect(&self.control_address)).await {
                Ok(stream) => stream?,
                Err(_) => return Err(EnvoyError::TCPConnectionError),
            };

        let mut response_stream =
            match tokio::time::timeout(timeout, TcpStream::connect(&self.response_address)).await {
                Ok(stream) => stream?,
                Err(_) => return Err(EnvoyError::TCPConnectionError),
            };

        loop {
            tokio::select! {
                _ = self.cancel.recv() => {
                    return Ok(())
                }

                data = self.incoming.recv() => {
                    if let Some(message) = data {
                        let response = self.submit_operation(message, &mut control_stream, &mut response_stream).await?;
                        self.outgoing.send(response).await?;
                    } else {
                        return Ok(())
                    }
                }
            }
        }
    }

    async fn submit_operation(
        &mut self,
        message: EmbassyMessage,
        control_stream: &mut TcpStream,
        response_stream: &mut TcpStream,
    ) -> Result<EmbassyMessage, EnvoyError> {
        control_stream
            .write_all(message.operation.as_bytes())
            .await?;
        let mut response = String::new();
        response_stream.read_to_string(&mut response).await?;

        let mut status = FribStatus::Failed;
        if response.contains("OK") {
            status = FribStatus::Ok;
        } else if response.contains("ERROR") {
            status = FribStatus::Errored;
        }

        return Ok(EmbassyMessage::compose_frib_response(status.to_string(), 0));
    }
}

pub fn startup_frib_envoy(
    runtime: &mut tokio::runtime::Runtime,
    frib_tx: &mpsc::Sender<EmbassyMessage>,
    cancel: &broadcast::Sender<EmbassyMessage>,
    control_port: i32,
    response_port: i32,
) -> (JoinHandle<()>, mpsc::Sender<EmbassyMessage>) {
    let (embassy_tx, frib_rx) = mpsc::channel::<EmbassyMessage>(10);
    let this_frib_tx = frib_tx.clone();
    let this_cancel = cancel.subscribe();
    let handle = runtime.spawn(async move {
        match FribEnvoy::new(
            control_port,
            response_port,
            frib_rx,
            this_frib_tx,
            this_cancel,
        ) {
            Ok(mut ev) => match ev.wait_for_operation().await {
                Ok(()) => (),
                Err(e) => tracing::error!("FRIBDAQ operation envoy ran into an error: {}", e),
            },
            Err(e) => tracing::error!("Error creating FRIBDAQ operation envoy: {}", e),
        }
    });
    return (handle, embassy_tx);
}
