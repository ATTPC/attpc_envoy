use super::constants::{
    ADDRESS_START, DATA_EXPORTER_LISTENER_PORT, DATA_ROUTER_LISTENER_PORT, MUTANT_ID,
    NUMBER_OF_MODULES, PROTOCOL,
};
use super::ecc_operation::ECCOperation;
use super::error::EnvoyError;
use super::message::EmbassyMessage;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

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

/// Response type for ECC Status query
/// Native format is XML
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct ECCStatusResponse {
    pub error_code: i32,
    pub error_message: String,
    pub state: i32,
    pub transition: i32,
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

/// The structure encompassing an async task associated with the ECC Server system.
/// ECCEnvoys have two modes, status check and transition. Transition envoys tell the server
/// when to load/unload configuration data. Status check envoys simply check the status
/// of the server every few seconds. In principle these tasks could be combined using a timer,
/// however, I think that separate tasks are prefered to keep the system as reactive as possible
#[derive(Debug)]
pub struct ECCEnvoy {
    config: ECCConfig,
    connection: Client,
    incoming: mpsc::Receiver<EmbassyMessage>,
    outgoing: mpsc::Sender<EmbassyMessage>,
    cancel: broadcast::Receiver<EmbassyMessage>,
}

impl ECCEnvoy {
    pub fn new(
        config: ECCConfig,
        rx: mpsc::Receiver<EmbassyMessage>,
        tx: mpsc::Sender<EmbassyMessage>,
        cancel: broadcast::Receiver<EmbassyMessage>,
    ) -> Result<Self, EnvoyError> {
        //120s (2min) default timeouts to match ECCServer/Client
        let connection_out = Duration::from_secs(120);
        let req_timeout = Duration::from_secs(120);

        //Probably need some options here, for now just set some timeouts
        let client = Client::builder()
            .connect_timeout(connection_out)
            .timeout(req_timeout)
            .build()?;
        Ok(Self {
            config,
            connection: client,
            incoming: rx,
            outgoing: tx,
            cancel,
        })
    }

    /// This one of the core task loops for an ECCEnvoy. Waits for a
    /// message from the embassy to transition the configuration of
    /// an ECC Server. Uses tokio::select! to handle cancelling
    pub async fn wait_for_transition(&mut self) -> Result<(), EnvoyError> {
        loop {
            tokio::select! {
                _ = self.cancel.recv() => {
                    return Ok(())
                }

                data = self.incoming.recv() => {
                    if let Some(message) = data {
                        let response = self.submit_transition(message).await?;
                        self.outgoing.send(response).await?;
                    } else {
                        return Ok(())
                    }
                }
            }
        }
    }

    /// This one of the core task loops for an ECCEnvoy. Every two seconds check the
    /// status of the ECC Server. Uses tokio::select! to handle cancelling.
    pub async fn wait_check_status(&mut self) -> Result<(), EnvoyError> {
        loop {
            tokio::select! {
                _ = self.cancel.recv() => {
                    return Ok(());
                }

                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    if let Ok(response) = self.submit_check_status().await {
                        self.outgoing.send(response).await?
                    } else {
                        let response = ECCStatusResponse { error_code: 0, error_message: String::from(""), state: 0, transition: 0 };
                        let message = EmbassyMessage::compose_ecc_status(serde_yaml::to_string(&response)?, self.config.id);
                        self.outgoing.send(message).await?
                    }
                }
            }
        }
    }

    /// Submit a transition request to the associated getECCServer
    /// and parse the response
    async fn submit_transition(
        &self,
        message: EmbassyMessage,
    ) -> Result<EmbassyMessage, EnvoyError> {
        let ecc_message = self.compose_ecc_transition_request(message)?;
        let response = self
            .connection
            .post(&self.config.url)
            .header("ContentType", "text/xml")
            .body(ecc_message)
            .send()
            .await?;
        let parsed_response = self.parse_ecc_operation_response(response).await?;
        Ok(parsed_response)
    }

    /// Submit a status check request to the associated getECCServer
    /// and parse the response
    async fn submit_check_status(&self) -> Result<EmbassyMessage, EnvoyError> {
        let message = format!("{ECC_SOAP_HEADER}<GetState>\n</GetState>\n{ECC_SOAP_FOOTER}");
        let response = self
            .connection
            .post(&self.config.url)
            .header("ContentType", "text/xml")
            .body(message)
            .send()
            .await?;
        let parsed_response = self.parse_ecc_status_response(response).await?;
        Ok(parsed_response)
    }

    /// Parse an ECC operation (transition) response and compose the
    /// appropriate EmbassyMessage
    async fn parse_ecc_operation_response(
        &self,
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

        Ok(EmbassyMessage::compose_ecc_response(
            serde_yaml::to_string(&parsed)?,
            self.config.id,
        ))
    }

    /// Parse an ECC status response and compose the appropriate
    /// EmbassyMessage
    async fn parse_ecc_status_response(
        &self,
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

        let status_response =
            EmbassyMessage::compose_ecc_status(serde_yaml::to_string(&parsed)?, self.config.id);
        Ok(status_response)
    }

    /// Compose the xml string for the given transition request
    fn compose_ecc_transition_request(
        &self,
        message: EmbassyMessage,
    ) -> Result<String, EnvoyError> {
        let op = ECCOperation::try_from(message.operation)?;
        let config = self.config.compose_config_body();
        let link = self.config.compose_data_link_body();
        Ok(format!(
            "{ECC_SOAP_HEADER}<{op}>\n{config}{link}</{op}>\n{ECC_SOAP_FOOTER}"
        ))
    }
}

/// Startup the ECC communication system
/// Takes in a runtime reference, experiment name, and a channel to send data to the embassy. Spawns the ECCEnvoys with tasks to either wait for
/// a command to transition that ECC DAQ or to periodically check the status of that particular ECC DAQ.
pub fn startup_ecc_envoys(
    runtime: &mut tokio::runtime::Runtime,
    experiment: &str,
    ecc_tx: &mpsc::Sender<EmbassyMessage>,
    cancel: &broadcast::Sender<EmbassyMessage>,
) -> (
    Vec<JoinHandle<()>>,
    HashMap<usize, mpsc::Sender<EmbassyMessage>>,
) {
    let mut transition_switchboard = HashMap::new();
    let mut handles: Vec<JoinHandle<()>> = vec![];

    //spin up the transition envoys
    for id in 0..NUMBER_OF_MODULES {
        let config = ECCConfig::new(id, experiment);
        let (embassy_tx, ecc_rx) = mpsc::channel::<EmbassyMessage>(10);
        let this_ecc_tx = ecc_tx.clone();
        let this_cancel = cancel.subscribe();
        let handle = runtime.spawn(async move {
            match ECCEnvoy::new(config, ecc_rx, this_ecc_tx, this_cancel) {
                Ok(mut ev) => match ev.wait_for_transition().await {
                    Ok(()) => (),
                    Err(e) => tracing::error!("ECC transition envoy ran into an error: {}", e),
                },
                Err(e) => tracing::error!("Error creating ECC transition envoy: {}", e),
            }
        });

        transition_switchboard.insert(id, embassy_tx);
        handles.push(handle);
    }

    //spin up the status envoys
    for id in 0..NUMBER_OF_MODULES {
        let config = ECCConfig::new(id, experiment);
        //The incoming channel is unused in the status envoy, however this may be changed later.
        //Could be useful to tie the update rate to the GUI?
        let (_, ecc_rx) = mpsc::channel::<EmbassyMessage>(10);
        let this_ecc_tx = ecc_tx.clone();
        let this_cancel = cancel.subscribe();
        let handle = runtime.spawn(async move {
            match ECCEnvoy::new(config, ecc_rx, this_ecc_tx, this_cancel) {
                Ok(mut ev) => match ev.wait_check_status().await {
                    Ok(()) => (),
                    Err(e) => tracing::error!("ECC status envoy ran into an error: {}", e),
                },
                Err(e) => tracing::error!("Error creating ECC status envoy: {}", e),
            }
        });

        handles.push(handle);
    }

    (handles, transition_switchboard)
}
