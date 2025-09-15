use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    default,
    env::{self, VarError},
    fmt::Error,
    sync::Arc,
};

use bollard::{
    Docker,
    models::{ContainerCreateBody, HostConfig, PortBinding},
    query_parameters::CreateContainerOptionsBuilder,
};
use serde::Serialize;

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

struct PortAllocator {
    available_ports: BinaryHeap<Reverse<u16>>,
    active_ports: HashSet<u16>,
    pub next_port: u16,
}

impl PortAllocator {
    pub fn new(starting_port: u16) -> Self {
        Self {
            available_ports: BinaryHeap::new(),
            active_ports: HashSet::new(),
            next_port: starting_port,
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

pub struct ContainerCreationResult {
    pub id: String,
    pub wan_ip: String,
    pub lan_ip: String,
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
    config: ChungustratorConfig,
    client: Docker,
    list: HashMap<String, String>,
    port_allocator: PortAllocator,
}

impl Chungustrator {
    pub fn new() -> Result<Self, bollard::errors::Error> {
        let config = ChungustratorConfig::new().unwrap_or_else(|_| ChungustratorConfig {
            wan_ip: "".to_string(),
            lan_ip: "".to_string(),
        });

        let client = Docker::connect_with_socket_defaults()?;

        Ok(Self {
            config,
            client,
            list: HashMap::new(),
            port_allocator: PortAllocator::new(28785),
        })
    }

    pub async fn create_container(
        &mut self,
    ) -> Result<ContainerCreationResult, bollard::errors::Error> {
        let ports = self.port_allocator.allocate_port();
        let port = ports.game_server_port;

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
                                    format!("{}/tcp", port),
                                    Some(vec![PortBinding {
                                        host_ip: Some("0.0.0.0".to_string()),
                                        host_port: Some(port.to_string()),
                                    }]),
                                );
                                port_bindings.insert(
                                    format!("{}/udp", port),
                                    Some(vec![PortBinding {
                                        host_ip: Some("0.0.0.0".to_string()),
                                        host_port: Some(port.to_string()),
                                    }]),
                                );
                                port_bindings
                            }),
                            network_mode: Some("vidya_chunguswork".to_string()),
                            ..Default::default()
                        }
                    }),
                    image: Some("chungusmod:latest".to_string()),
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
                            format!("GAME_SERVER_PORT={}", port),
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

        Ok(ContainerCreationResult {
            id: game_server_container_id,
            wan_ip: self.config.wan_ip.clone(),
            lan_ip: self.config.lan_ip.clone(),
        })
    }
}
