use super::constants::{ADDRESS_START, NUMBER_OF_MODULES};
use super::error::EnvoyError;
use super::message::EmbassyMessage;
use super::sentry_types::{SentryOperation, SentryResponse, SentryStatus};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const SENTRY_PORT: i32 = 8080;
const STATUS_WAIT_TIME_SEC: u64 = 2;
const CONNECTION_TIMEOUT_SEC: u64 = 120;

pub struct SentryConfig {
    id: usize,
    base_address: String,
}

impl SentryConfig {
    pub fn new(id: usize) -> Self {
        let base_address = format!("http://{}.{}:{}", ADDRESS_START, 60 + id, SENTRY_PORT);

        Self { id, base_address }
    }

    pub fn status(&self) -> String {
        format!("{}/status", self.base_address)
    }

    pub fn catalog(&self) -> String {
        format!("{}/catalog", self.base_address)
    }

    pub fn backup(&self) -> String {
        format!("{}/backup", self.base_address)
    }
}

pub async fn run_sentry_envoy(
    config: SentryConfig,
    mut incoming: broadcast::Receiver<EmbassyMessage>,
    outgoing: mpsc::Sender<EmbassyMessage>,
    mut cancel: broadcast::Receiver<EmbassyMessage>,
) -> Result<(), EnvoyError> {
    let mut prev_written_gb: f64 = 0.0;
    let connection_out = Duration::from_secs(CONNECTION_TIMEOUT_SEC);
    let req_timeout = Duration::from_secs(CONNECTION_TIMEOUT_SEC);

    //Probably need some options here, for now just set some timeouts
    let client = Client::builder()
        .connect_timeout(connection_out)
        .timeout(req_timeout)
        .build()?;

    loop {
        tokio::select! {
            _ = cancel.recv() => {
                return Ok(());
            }
            maybe = incoming.recv() => {
                if let Ok(msg) = maybe {
                    let operation: SentryOperation = serde_json::from_str(&msg.body)?;
                    let response = submit_operation(&client, &config, operation, &mut prev_written_gb).await?;
                    outgoing.send(response).await?;
                } else {
                    return Ok(());
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(STATUS_WAIT_TIME_SEC)) => {
                let response = submit_check_status(&client, &config, &mut prev_written_gb).await?;
                outgoing.send(response).await?;
            }
        }
    }
}

async fn submit_operation(
    client: &Client,
    config: &SentryConfig,
    operation: SentryOperation,
    prev_written_gb: &mut f64,
) -> Result<EmbassyMessage, EnvoyError> {
    let response = match operation {
        SentryOperation::Backup(params) => {
            *prev_written_gb = 0.0;
            client.post(config.backup()).json(&params).send().await?
        }
        SentryOperation::Catalog(params) => {
            client.post(config.catalog()).json(&params).send().await?
        }
    };
    match response.status() {
        StatusCode::INTERNAL_SERVER_ERROR => {
            return Err(EnvoyError::ServerError(response.text().await?))
        }
        _ => (),
    }
    let resp_string = response.text().await?;
    let parsed: SentryResponse = serde_json::from_str(&resp_string)?;
    let status = SentryStatus::from_response(parsed, &0.0, 1.0);
    Ok(EmbassyMessage::compose(status, config.id))
}

async fn submit_check_status(
    client: &Client,
    config: &SentryConfig,
    prev_written_gb: &mut f64,
) -> Result<EmbassyMessage, EnvoyError> {
    let response = client.get(config.status()).send().await?;

    match response.status() {
        StatusCode::INTERNAL_SERVER_ERROR => {
            tracing::error!(
                "SentryEnvoy received a server error when checking status: {}",
                response.text().await?
            );
            return Ok(EmbassyMessage::compose(SentryStatus::default(), config.id));
        }
        _ => (),
    }

    let resp_string = response.text().await?;
    let parsed: SentryResponse = serde_json::from_str(&resp_string)?;
    let current_path_gb = parsed.data_written_gb;
    let status = SentryStatus::from_response(parsed, prev_written_gb, STATUS_WAIT_TIME_SEC as f64);
    *prev_written_gb += current_path_gb;
    Ok(EmbassyMessage::compose(status, config.id))
}

pub fn startup_sentry_envoys(
    runtime: &mut tokio::runtime::Runtime,
    tx: &mpsc::Sender<EmbassyMessage>,
    operation: &broadcast::Sender<EmbassyMessage>,
    cancel: &broadcast::Sender<EmbassyMessage>,
) -> Vec<JoinHandle<()>> {
    let mut handles: Vec<JoinHandle<()>> = vec![];

    //spin up the envoys
    for id in 0..NUMBER_OF_MODULES {
        let config = SentryConfig::new(id);
        let this_tx = tx.clone();
        let this_cancel = cancel.subscribe();
        let this_op = operation.subscribe();
        let handle = runtime.spawn(async move {
            match run_sentry_envoy(config, this_op, this_tx, this_cancel).await {
                Ok(()) => (),
                Err(e) => tracing::error!("Error in Sentry envoy: {}", e),
            }
        });

        handles.push(handle);
    }

    handles
}
