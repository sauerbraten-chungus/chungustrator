use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    default,
    env::{self, VarError},
    fmt::Error,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use bollard::{
    Docker,
    models::{ContainerCreateBody, EndpointSettings, HostConfig, PortBinding},
    network::ConnectNetworkOptions,
    query_parameters::CreateContainerOptionsBuilder,
};
use serde::Serialize;

use thiserror::Error;
use tokio::{select, sync::mpsc, time};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::Channel;
use tracing::{error, info};

use crate::chungustrator_enet::{
    self, ChungustratorMessage, ChunguswayMessage, Ping, VerificationCodeRequest,
    chungus_service_client::ChungusServiceClient, chungustrator_message, chungusway_message,
};

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Error from docker: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("Error from tonic: {0}")]
    Tonic(#[from] tonic::Status),
}

pub enum OrchestratorMessage {
    CreateContainer {
        verification_codes: HashMap<String, String>,
        response_tx: mpsc::UnboundedSender<OrchestratorResponse>,
    },
}

pub enum OrchestratorResponse {
    ContainerCreationSuccess {
        id: String,
        wan_ip: String,
        lan_ip: String,
        port: u16,
    },
    ContainerCreationError {
        error: String,
    },
}

pub struct ServerPorts {
    game_server_port: u16,
    server_query_client_port: u16,
}

impl ServerPorts {
    pub fn new(port: u16) -> Self {
        Self {
            game_server_port: port,
            server_query_client_port: port + 1,
        }
    }

    pub fn all_ports(&self) -> [u16; 2] {
        [self.game_server_port, self.server_query_client_port]
    }
}

struct ServerAllocator {
    available_ports: BinaryHeap<Reverse<u16>>,
    active_ports: HashSet<u16>,
    pub next_port: u16,
    servers: HashMap<String, u16>,
}

impl ServerAllocator {
    pub fn new(starting_port: u16) -> Self {
        Self {
            available_ports: BinaryHeap::new(),
            active_ports: HashSet::new(),
            next_port: starting_port,
            servers: HashMap::new(),
        }
    }

    pub fn allocate_port(&mut self) -> ServerPorts {
        let base_port = if let Some(Reverse(port)) = self.available_ports.pop() {
            port
        } else {
            let port = self.next_port;
            self.next_port += 2;
            port
        };

        println!("{}", base_port);

        let server_ports = ServerPorts::new(base_port);

        for ports in server_ports.all_ports() {
            self.active_ports.insert(ports);
        }

        server_ports
    }

    pub fn release_port(&mut self, server_ports: ServerPorts) -> bool {
        let game_server_port = server_ports.game_server_port;

        for ports in server_ports.all_ports() {
            if !self.active_ports.remove(&ports) {
                return false;
            }
        }
        self.available_ports.push(Reverse(game_server_port));

        true
    }

    pub fn add_server(&mut self, container_id: String, port: u16) {
        self.servers.insert(container_id, port);
    }

    pub fn remove_server(&mut self, container_id: String) {
        self.servers.remove(&container_id);
    }

    pub fn is_port_active(&self, port: u16) -> bool {
        if self.active_ports.contains(&port) {
            true
        } else {
            false
        }
    }

    pub fn allocated_port_count(&self) -> usize {
        self.active_ports.len()
    }
}

struct ChungustratorConfig {
    wan_ip: String,
    lan_ip: String,
}

impl ChungustratorConfig {
    pub fn new() -> Result<Self, VarError> {
        Ok(Self {
            wan_ip: env::var("WAN_IP")?,
            lan_ip: env::var("LAN_IP")?,
        })
    }
}

pub struct Chungustrator {
    stream_tx: mpsc::UnboundedSender<ChungustratorMessage>,
    config: ChungustratorConfig,
    client: Docker,
    list: HashMap<String, String>,
    server_allocator: ServerAllocator,
    rx: mpsc::UnboundedReceiver<OrchestratorMessage>,
}

impl Chungustrator {
    pub async fn new(
        rx: mpsc::UnboundedReceiver<OrchestratorMessage>,
        mut chungus_stub: ChungusServiceClient<Channel>,
    ) -> Result<(), OrchestratorError> {
        let config = ChungustratorConfig::new().unwrap_or_else(|_| ChungustratorConfig {
            wan_ip: "".to_string(),
            lan_ip: "".to_string(),
        });

        // Create channel for outgoing stream messages
        let (stream_tx, stream_rx) = mpsc::unbounded_channel::<ChungustratorMessage>();

        // Convert receiver to stream
        let outbound_stream = UnboundedReceiverStream::new(stream_rx);

        // Send initial ping to establish connection
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        stream_tx
            .send(ChungustratorMessage {
                payload: Some(chungustrator_message::Payload::Ping(Ping { timestamp })),
            })
            .ok();

        info!("Sent initial ping with timestamp: {}", timestamp);

        // Establish bidirectional streaming connection
        let response = chungus_stub.stream_events(outbound_stream).await?;
        let mut inbound_stream = response.into_inner();

        // Spawn task to handle incoming messages from the service
        tokio::spawn(async move {
            while let Ok(Some(msg)) = inbound_stream.message().await {
                Self::handle_incoming_message(msg).await;
            }
            info!("Chungus stream closed");
        });

        let client = Docker::connect_with_socket_defaults()?;
        let orchestrator = Chungustrator {
            stream_tx,
            config,
            client,
            list: HashMap::new(),
            server_allocator: ServerAllocator::new(28785),
            rx,
        };

        tokio::spawn(async move { orchestrator.run().await });

        Ok(())
    }

    async fn handle_incoming_message(msg: ChunguswayMessage) {
        if let Some(payload) = msg.payload {
            match payload {
                chungusway_message::Payload::VerificationCodeRes(response) => {
                    info!("Received verification code response: {}", response.msg);
                }
                chungusway_message::Payload::Shutdown(shutdown) => {
                    info!(
                        "Received shutdown event for server {} at {} - reason: {}",
                        shutdown.server_address, shutdown.timestamp, shutdown.reason
                    );
                }
                chungusway_message::Payload::Pong(pong) => {
                    info!("Received pong at timestamp: {}", pong.timestamp);
                }
            }
        }
    }

    async fn run(mut self) {
        let mut interval = time::interval(time::Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("processing internal shit");
                }
                Some(msg) = self.rx.recv() => {
                    info!("received msg from handler!");
                    if let Err(e) = self.receive(msg).await {
                        error!("Orchestrator Error: {}", e);
                    }
                }
            }
        }
    }

    async fn receive(&mut self, msg: OrchestratorMessage) -> Result<(), OrchestratorError> {
        match msg {
            OrchestratorMessage::CreateContainer {
                verification_codes,
                response_tx,
            } => {
                self.create_container(verification_codes, response_tx)
                    .await?;
            }
        }
        Ok(())
    }

    async fn do_create_container(
        &mut self,
        game_server_port: u16,
        server_query_client_port: u16,
    ) -> Result<String, OrchestratorError> {
        let game_server_container_id = self
            .client
            .create_container(
                None::<bollard::query_parameters::CreateContainerOptions>,
                ContainerCreateBody {
                    host_config: Some({
                        HostConfig {
                            port_bindings: Some({
                                let mut port_bindings =
                                    HashMap::<String, Option<Vec<PortBinding>>>::new();
                                port_bindings.insert(
                                    format!("{}/tcp", game_server_port),
                                    Some(vec![PortBinding {
                                        host_ip: Some("0.0.0.0".to_string()),
                                        host_port: Some(game_server_port.to_string()),
                                    }]),
                                );
                                port_bindings.insert(
                                    format!("{}/udp", game_server_port),
                                    Some(vec![PortBinding {
                                        host_ip: Some("0.0.0.0".to_string()),
                                        host_port: Some(game_server_port.to_string()),
                                    }]),
                                );
                                port_bindings
                            }),
                            // network_mode: Some("vidya_chunguswork".to_string()),
                            extra_hosts: Some(vec![
                                "host.docker.internal:host-gateway".to_string(),
                            ]),
                            ..Default::default()
                        }
                    }),
                    image: Some("chungusmod:latest".to_string()),
                    env: Some(vec![
                        format!("GAME_SERVER_PORT={}", game_server_port),
                        "CHUNGUS_PEER_ADDRESS=host.docker.internal".to_string(),
                        "CHUNGUS_PEER_PORT=30000".to_string(),
                    ]),
                    exposed_ports: Some({
                        let mut exposed_ports = HashMap::new();
                        exposed_ports.insert(format!("{}/tcp", game_server_port), HashMap::new());
                        exposed_ports.insert(format!("{}/udp", game_server_port), HashMap::new());
                        exposed_ports.insert(
                            format!("{}/tcp", server_query_client_port.to_string()),
                            HashMap::new(),
                        );
                        exposed_ports.insert(
                            format!("{}/udp", server_query_client_port.to_string()),
                            HashMap::new(),
                        );
                        exposed_ports
                    }),
                    ..Default::default()
                },
            )
            .await?
            .id;

        let server_query_client_id = self
            .client
            .create_container(
                None::<bollard::query_parameters::CreateContainerOptions>,
                ContainerCreateBody {
                    host_config: Some({
                        HostConfig {
                            network_mode: Some(format!("container:{}", game_server_container_id)),
                            ..Default::default()
                        }
                    }),
                    image: Some("sqc:latest".to_string()),
                    env: Some({
                        vec![
                            "PLAYER_SERVICE_IP=http://player:3000".to_string(),
                            "AUTH_SERVICE_IP=http://auth:8081".to_string(),
                            "GAME_SERVER_IP=localhost".to_string(),
                            format!("GAME_SERVER_PORT={}", game_server_port),
                            format!(
                                "SECRET_CHUNGUS={}",
                                env::var("SECRET_CHUNGUS").unwrap_or_default()
                            ),
                            format!(
                                "CHUNGUS_KEY={}",
                                env::var("CHUNGUS_KEY").unwrap_or_default()
                            ),
                        ]
                    }),
                    ..Default::default()
                },
            )
            .await?
            .id;

        self.client
            .start_container(
                &game_server_container_id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await?;

        self.client
            .start_container(
                &server_query_client_id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await?;

        Ok(game_server_container_id)
    }

    pub async fn create_container(
        &mut self,
        verification_codes: HashMap<String, String>,
        response_tx: mpsc::UnboundedSender<OrchestratorResponse>,
    ) -> Result<(), OrchestratorError> {
        let ports = self.server_allocator.allocate_port();
        let game_server_port = ports.game_server_port;
        let server_query_client_port = ports.server_query_client_port;

        let container_id = match self
            .do_create_container(game_server_port, server_query_client_port)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                self.server_allocator.release_port(ports);
                return Err(e.into());
            }
        };

        if let Err(e) = response_tx.send(OrchestratorResponse::ContainerCreationSuccess {
            id: container_id,
            wan_ip: self.config.wan_ip.clone(),
            lan_ip: self.config.lan_ip.clone(),
            port: game_server_port,
        }) {
            error!("Channel error sending create container response: {}", e);
        }

        // Send verification codes through the stream
        let message = ChungustratorMessage {
            payload: Some(chungustrator_message::Payload::VerificationCodeReq(
                VerificationCodeRequest {
                    codes: verification_codes,
                },
            )),
        };

        if let Err(e) = self.stream_tx.send(message) {
            error!("Failed to send verification codes through stream: {}", e);
        }

        Ok(())
    }
}
