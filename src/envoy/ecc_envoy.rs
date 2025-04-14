use super::constants::{
    ADDRESS_START, DATA_EXPORTER_LISTENER_PORT, DATA_ROUTER_LISTENER_PORT, MUTANT_ID,
    NUMBER_OF_MODULES, PROTOCOL,
};
use super::ecc_operation::ECCOperation;
use super::error::EnvoyError;
use super::message::{EmbassyMessage, MessageKind, ToMessage};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Amount of time to wait to check status
const STATUS_WAIT_TIME_SEC: u64 = 2;

/// Connection timeout
const CONNECTION_TIMEOUT_SEC: u64 = 120;

/// The default port for ECC
const ECC_URL_PORT: i32 = 8083;

/// The SOAP protocol header for ECC
const ECC_SOAP_HEADER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
    <SOAP-ENV:Envelope 
    xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" 
    xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xmlns:xsd="http://www.w3.org/2001/XMLSchema"
    xmlns="urn:ecc">
    <SOAP-ENV:Body>
"#;

const ECC_SOAP_FOOTER: &str = r#"
    </SOAP-ENV:Body>
    </SOAP-ENV:Envelope>
"#;

/// Response type for ECC Operations (transitions)
/// Native format is XML
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ECCOperationResponse {
    pub error_code: i32,
    pub error_message: String,
    pub text: String,
}

impl ToMessage for ECCOperationResponse {
    fn message_kind(&self) -> MessageKind {
        MessageKind::ECCOpResponse
    }
}

/// Response type for ECC Status query
/// Native format is XML
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ECCStatusResponse {
    pub error_code: i32,
    pub error_message: String,
    pub state: i32,
    pub transition: i32,
}

impl ToMessage for ECCStatusResponse {
    fn message_kind(&self) -> MessageKind {
        MessageKind::ECCStatus
    }
}

/// Struct defining a minimal getECCServer configuration
#[derive(Debug, Clone)]
pub struct ECCConfig {
    id: usize,
    experiment: String,
    address: String,
    url: String,
}

impl ECCConfig {
    /// Create a ECC config from an experiment name and module ID
    pub fn new(id: usize, experiment: &str) -> ECCConfig {
        let address = match id {
            MUTANT_ID => format!("{ADDRESS_START}.1"),
            _ => format!("{ADDRESS_START}.{}", 60 + id),
        };
        let url = Self::url(&address);
        ECCConfig {
            id,
            experiment: experiment.to_string(),
            address,
            url,
        }
    }

    /// Compse the xml string defining the ECC configuration
    fn compose_config_body(&self) -> String {
        let describe = self.describe();
        let prepare = self.experiment.clone();
        let configure = self.experiment.clone();
        format!(
            r#"<configID>
                        <ConfigId>
                            <SubConfigId type="describe">
                                {describe}
                            </SubConfigId>
                            <SubConfigId type="prepare">
                                {prepare}
                            </SubConfigId>
                            <SubConfigId type="configure">
                                {configure}
                            </SubConfigId>
                        </ConfigId>
                    </configID>"#
        )
    }

    /// Compose the xml string defining the ECC data link (i.e. connections to DataRouters)
    /// We define two routers: the file dump (DataRouter) and export (DataExporter)
    fn compose_data_link_body(&self) -> String {
        let source = self.source();
        let ip = self.address.clone();
        let router = self.data_router();
        let exporter = self.data_exporter();
        format!(
            r#"<table>
                        <DataLinkSet>
                            <DataLink>
                                <DataSender id="{source}" />
                                <DataRouter ipAddress="{ip}" name="{router}" port="{DATA_ROUTER_LISTENER_PORT}" type="{PROTOCOL}" />
                            </DataLink>
                            <DataLink>
                                <DataSender id="{source}" />
                                <DataRouter ipAddress="{ip}" name="{exporter}" port="{DATA_EXPORTER_LISTENER_PORT}" type="{PROTOCOL}" />
                            </DataLink>
                        </DataLinkSet>
                    </table>"#
        )
    }

    /// Comopose the string defining the describe ID
    fn describe(&self) -> String {
        match self.id {
            MUTANT_ID => self.experiment.clone(),
            _ => format!("cobo{}", self.id),
        }
    }

    /// Compose the string defining the data source (module)
    fn source(&self) -> String {
        match self.id {
            MUTANT_ID => String::from("Mutant[master]"),
            _ => format!("CoBo[{}]", self.id),
        }
    }

    /// Compose the string defining the associated DataRouter
    fn data_router(&self) -> String {
        format!("data{}", self.id)
    }

    /// Compose the string defining the associated DataExporter
    fn data_exporter(&self) -> String {
        format!("exporter{}", self.id)
    }

    /// Compose the associated getECCServer URL
    fn url(address: &str) -> String {
        format!("http://{}:{}", address, ECC_URL_PORT)
    }
}

/// Run an ECC envoy, communicating with the ECCServer
async fn run_ecc_envoy(
    config: ECCConfig,
    mut incoming: mpsc::Receiver<EmbassyMessage>,
    outgoing: mpsc::Sender<EmbassyMessage>,
    mut cancel: broadcast::Receiver<EmbassyMessage>,
) -> Result<(), EnvoyError> {
    let connection_out = Duration::from_secs(CONNECTION_TIMEOUT_SEC);
    let req_timeout = Duration::from_secs(CONNECTION_TIMEOUT_SEC);

    //Probably need some options here, for now just set some timeouts
    let client = Client::builder()
        .connect_timeout(connection_out)
        .timeout(req_timeout)
        .build()?;
    // This is the core loop of the envoy. Wait for one of three conditions.
    // 1. A cancel message. This stops the envoy and ends the task
    // 2. A operation (ECCOperation) has been requested. Submit the request to the module
    // 3. 2 seconds pass. Every 2 sec query the status of the server.
    loop {
        tokio::select! {
            _ = cancel.recv() => {
                return Ok(())
            }

            data = incoming.recv() => {
                if let Some(message) = data {
                    match submit_operation(&config, &client, message).await {
                        Ok(response) => outgoing.send(response).await?,
                        Err(e) => tracing::warn!("ECC failed to submit operation: {e}"),
                    }
                } else {
                    return Ok(())
                }
            }

            _ = tokio::time::sleep(Duration::from_secs(STATUS_WAIT_TIME_SEC)) => {
                if let Ok(response) = submit_check_status(&config, &client).await {
                    outgoing.send(response).await?
                } else {
                    let response = ECCStatusResponse { error_code: 0, error_message: String::from(""), state: 0, transition: 0 };
                    let message = EmbassyMessage::compose(response, config.id);
                    outgoing.send(message).await?
                }
            }
        }
    }
}

/// Submit a operation (ECCOperation)
async fn submit_operation(
    config: &ECCConfig,
    cxn: &Client,
    message: EmbassyMessage,
) -> Result<EmbassyMessage, EnvoyError> {
    let ecc_message = compose_operation_request(config, message)?;
    let response = cxn
        .post(&config.url)
        .header("ContentType", "text/xml")
        .body(ecc_message)
        .send()
        .await?;
    let parsed_response = parse_operation_response(config, response).await?;
    Ok(parsed_response)
}

/// Sumbit a status check
async fn submit_check_status(
    config: &ECCConfig,
    cxn: &Client,
) -> Result<EmbassyMessage, EnvoyError> {
    let message = format!("{ECC_SOAP_HEADER}<GetState>\n</GetState>\n{ECC_SOAP_FOOTER}");
    let response = cxn
        .post(&config.url)
        .header("ContentType", "text/xml")
        .body(message)
        .send()
        .await?;
    let parsed_response = parse_status_response(config, response).await?;
    Ok(parsed_response)
}

/// Compose the operation request (text)
fn compose_operation_request(
    config: &ECCConfig,
    message: EmbassyMessage,
) -> Result<String, EnvoyError> {
    let op: ECCOperation = serde_json::from_str(&message.body)?;
    let body = config.compose_config_body();
    let link = config.compose_data_link_body();
    Ok(format!(
        "{ECC_SOAP_HEADER}<{op}>\n{body}{link}</{op}>\n{ECC_SOAP_FOOTER}"
    ))
}

/// Parse the response from the server after an operation
async fn parse_operation_response(
    config: &ECCConfig,
    response: Response,
) -> Result<EmbassyMessage, EnvoyError> {
    let text = response.text().await?;
    let mut reader = quick_xml::Reader::from_str(&text);
    let mut parsed = ECCOperationResponse::default();

    reader.read_event()?; //Opening
    reader.read_event()?; //Junk
    reader.read_event()?; //SOAP Decl
    reader.read_event()?; //SOAP Body
    reader.read_event()?; //ECC
    reader.read_event()?; //ErrorCode start tag
    let event = reader.read_event()?; //ErrorCode payload
    parsed.error_code = match event {
        quick_xml::events::Event::Text(t) => String::from_utf8(t.to_vec())?.parse()?,
        _ => return Err(EnvoyError::FailedXMLConvert),
    };
    reader.read_event()?; //ErrorCode end tag
    reader.read_event()?; //ErrorMesage start tag
    let event = reader.read_event()?; //ErrorMessage payload or end tag
    let mut is_msg = true;
    parsed.error_message = match event {
        quick_xml::events::Event::Text(t) => String::from_utf8(t.to_vec())?,
        _ => {
            is_msg = false;
            String::from("")
        }
    };
    if is_msg {
        reader.read_event()?; //ErrorMessage end tag
    }
    reader.read_event()?; //Text start tag
    let event = reader.read_event()?; //Text payload
    parsed.text = match event {
        quick_xml::events::Event::Text(t) => String::from_utf8(t.to_vec())?,
        _ => String::from(""),
    };

    Ok(EmbassyMessage::compose(parsed, config.id))
}

/// Parse the server status response
async fn parse_status_response(
    config: &ECCConfig,
    response: Response,
) -> Result<EmbassyMessage, EnvoyError> {
    let text = response.text().await?;
    let mut reader = quick_xml::Reader::from_str(&text);
    let mut parsed: ECCStatusResponse = ECCStatusResponse::default();

    reader.read_event()?; //Opening
    reader.read_event()?; //Junk
    reader.read_event()?; //SOAP Decl
    reader.read_event()?; //SOAP Body
    reader.read_event()?; //ECC
    reader.read_event()?; //ErrorCode start tag
    let event = reader.read_event()?; //ErrorCode payload
    parsed.error_code = match event {
        quick_xml::events::Event::Text(t) => String::from_utf8(t.to_vec())?.parse()?,
        _ => return Err(EnvoyError::FailedXMLConvert),
    };
    reader.read_event()?; //ErrorCode end tag
    reader.read_event()?; //ErrorMesage start tag
    let event = reader.read_event()?; //ErrorMessage payload or end tag
    let mut is_msg = true;
    parsed.error_message = match event {
        quick_xml::events::Event::Text(t) => String::from_utf8(t.to_vec())?,
        _ => {
            is_msg = false;
            String::from("")
        }
    };
    if is_msg {
        reader.read_event()?; //ErrorMessage end tag
    }
    reader.read_event()?; //State start tag
    let event = reader.read_event()?; //State payload
    parsed.state = match event {
        quick_xml::events::Event::Text(t) => String::from_utf8(t.to_vec())?.parse()?,
        _ => return Err(EnvoyError::FailedXMLConvert),
    };
    reader.read_event()?; //State end tag
    reader.read_event()?; //Transition start tag
    let event = reader.read_event()?; //Transition payload
    parsed.transition = match event {
        quick_xml::events::Event::Text(t) => String::from_utf8(t.to_vec())?.parse()?,
        _ => return Err(EnvoyError::FailedXMLConvert),
    };

    let status_response = EmbassyMessage::compose(parsed, config.id);
    Ok(status_response)
}

/// Startup the ECC communication system
/// Takes in a runtime reference, experiment name, and a channel to send data to the embassy. Spawns the ECCEnvoys with tasks to wait for
/// a command to operation that ECC DAQ and to periodically check the status of that particular ECC DAQ.
pub fn startup_ecc_envoys(
    runtime: &mut tokio::runtime::Runtime,
    experiment: &str,
    ecc_tx: &mpsc::Sender<EmbassyMessage>,
    cancel: &broadcast::Sender<EmbassyMessage>,
) -> (
    Vec<JoinHandle<()>>,
    HashMap<usize, mpsc::Sender<EmbassyMessage>>,
) {
    let mut switchboard = HashMap::new();
    let mut handles: Vec<JoinHandle<()>> = vec![];

    //spin up the envoys
    for id in 0..NUMBER_OF_MODULES {
        let config = ECCConfig::new(id, experiment);
        let (embassy_tx, ecc_rx) = mpsc::channel::<EmbassyMessage>(10);
        let this_ecc_tx = ecc_tx.clone();
        let this_cancel = cancel.subscribe();
        let handle = runtime.spawn(async move {
            match run_ecc_envoy(config, ecc_rx, this_ecc_tx, this_cancel).await {
                Ok(()) => (),
                Err(e) => tracing::error!("Error in ECC envoy: {}", e),
            }
        });

        switchboard.insert(id, embassy_tx);
        handles.push(handle);
    }

    (handles, switchboard)
}
