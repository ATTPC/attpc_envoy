use super::constants::{ADDRESS_START, NUMBER_OF_MODULES};
use super::error::EnvoyError;
use super::message::EmbassyMessage;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const SURVEYOR_URL_PORT: i32 = 8081;

const STATUS_WAIT_TIME_SEC: u64 = 2;

const CONNECTION_TIMEOUT_SEC: u64 = 120;

/// The message delivered from the SurveyorEnvoy (the status of a DataRouter and its machine)
/// Contains a lot of data from a lot of different pieces of the
/// filesystem on which the specific data router is running
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SurveyorResponse {
    pub state: i32,
    pub address: String,
    pub location: String,
    pub disk_status: String,
    pub percent_used: String,
    pub disk_space: u64,
    pub files: i32,
    pub bytes_used: u64,
    pub data_rate: f64,
}

impl Default for SurveyorResponse {
    fn default() -> Self {
        Self {
            state: 0,
            address: String::from("N/A"),
            location: String::from("N/A"),
            disk_status: String::from("N/A"),
            percent_used: String::from("N/A"),
            disk_space: 0,
            files: 0,
            bytes_used: 0,
            data_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SurveyorConfig {
    id: usize,
    address: String,
    url: String,
}

impl SurveyorConfig {
    pub fn new(id: usize) -> Self {
        let address = Self::address(&id);
        let url = Self::url(&address);

        Self { id, address, url }
    }

    fn address(id: &usize) -> String {
        format!("{ADDRESS_START}.{}", 60 + id)
    }

    fn url(address: &str) -> String {
        format!("http://{address}:{SURVEYOR_URL_PORT}/~attpc/surveyor.html")
    }
}

async fn run_surveyor_envoy(
    config: SurveyorConfig,
    outgoing: mpsc::Sender<EmbassyMessage>,
    mut cancel: broadcast::Receiver<EmbassyMessage>,
) -> Result<(), EnvoyError> {
    let mut previous_bytes: f64 = 0.0;
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

            _ = tokio::time::sleep(Duration::from_secs(STATUS_WAIT_TIME_SEC)) => {
                if let Ok(Some(response)) = submit_check_status(&config, &client, &mut previous_bytes).await {
                        outgoing.send(response).await?;
                } else {
                    let message = EmbassyMessage::compose_surveyor_response(serde_yaml::to_string(&SurveyorResponse::default())?, config.id);
                    outgoing.send(message).await?
                }
            }
        }
    }
}

async fn submit_check_status(
    config: &SurveyorConfig,
    cxn: &Client,
    previous_bytes: &mut f64,
) -> Result<Option<EmbassyMessage>, EnvoyError> {
    let response = cxn.get(&config.url).send().await?;
    parse_response(config, response, previous_bytes).await
}

async fn parse_response(
    config: &SurveyorConfig,
    response: Response,
    previous_bytes: &mut f64,
) -> Result<Option<EmbassyMessage>, EnvoyError> {
    let response_text = response.text().await?;
    let mut status = SurveyorResponse::default();
    let lines: Vec<&str> = response_text.lines().collect();

    if lines.is_empty() {
        return Ok(None);
    }

    status.state = lines[0].parse::<i32>()?;
    if status.state == 0 {
        return Ok(Some(EmbassyMessage::compose_surveyor_response(
            serde_yaml::to_string(&status)?,
            config.id,
        )));
    }
    status.address = config.address.clone();
    status.location = String::from(lines[1]);
    let line_entries: Vec<&str> = lines[3].split_whitespace().collect();
    status.percent_used = String::from(line_entries[4]);
    status.disk_space = line_entries[1].parse::<u64>()? * 512;

    let mut bytes: u64 = 0;
    let mut n_files = 0;
    for line in lines[4..].iter() {
        if line.contains("graw") {
            let line_entries: Vec<&str> = line.split_whitespace().collect();
            bytes += line_entries[4].parse::<u64>()?;
            n_files += 1;
        }
    }

    if n_files > 0 {
        status.disk_status = String::from("Filled");
    } else {
        status.disk_status = String::from("Empty");
    }

    status.files = n_files;
    status.bytes_used = bytes;
    let bytes_float = bytes as f64;

    status.data_rate = (bytes_float - *previous_bytes) * 1.0e-6 / (STATUS_WAIT_TIME_SEC as f64); //MB/s

    *previous_bytes = bytes_float;

    Ok(Some(EmbassyMessage::compose_surveyor_response(
        serde_yaml::to_string(&status)?,
        config.id,
    )))
}

/// Function to create all of the SurveyorEnvoys and spawn their tasks. Returns handles to the tasks.
pub fn startup_surveyor_envoys(
    runtime: &mut tokio::runtime::Runtime,
    surveyor_tx: &mpsc::Sender<EmbassyMessage>,
    cancel: &broadcast::Sender<EmbassyMessage>,
) -> Vec<JoinHandle<()>> {
    let mut handles: Vec<JoinHandle<()>> = vec![];

    //spin up the surveyor envoys, Mutant does not get a data router/surveyor
    for id in 0..(NUMBER_OF_MODULES - 1) {
        let config = SurveyorConfig::new(id);
        let this_surveyor_tx = surveyor_tx.clone();
        let this_cancel = cancel.subscribe();
        let handle = runtime.spawn(async move {
            match run_surveyor_envoy(config, this_surveyor_tx, this_cancel).await {
                Ok(()) => (),
                Err(e) => tracing::error!("SurveyorEnvoy had an error: {}", e),
            }
        });

        handles.push(handle);
    }

    handles
}
