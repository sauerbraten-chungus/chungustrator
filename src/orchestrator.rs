use std::{collections::HashMap, default, sync::Arc};

use bollard::{
    Docker,
    models::{ContainerCreateBody, HostConfig, PortBinding},
    query_parameters::CreateContainerOptionsBuilder,
};
use serde::Serialize;

pub struct Chungustrator {
    client: Docker,
    list: HashMap<String, String>,
}

impl Chungustrator {
    pub fn new() -> Result<Self, bollard::errors::Error> {
        let client = Docker::connect_with_socket_defaults()?;

        Ok(Self {
            client,
            list: HashMap::new(),
        })
    }

    pub async fn create_container(&self) -> Result<String, bollard::errors::Error> {
        let game_server_container_id = self
            .client
            .create_container(
                None::<bollard::query_parameters::CreateContainerOptions>,
                ContainerCreateBody {
                    // exposed_ports: Some({
                    //     let mut exposed_ports = HashMap::new();
                    //     exposed_ports.insert("28786/tcp".to_string(), HashMap::new());
                    //     exposed_ports.insert("28786/udp".to_string(), HashMap::new());
                    //     exposed_ports
                    // }),
                    host_config: Some({
                        HostConfig {
                            port_bindings: Some({
                                let mut port_bindings =
                                    HashMap::<String, Option<Vec<PortBinding>>>::new();
                                port_bindings.insert(
                                    "28785/tcp".to_string(),
                                    Some(vec![PortBinding {
                                        host_ip: Some("0.0.0.0".to_string()),
                                        host_port: Some("28785".to_string()),
                                    }]),
                                );
                                port_bindings.insert(
                                    "28785/udp".to_string(),
                                    Some(vec![PortBinding {
                                        host_ip: Some("0.0.0.0".to_string()),
                                        host_port: Some("28785".to_string()),
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
                    // exposed_ports: Some({
                    //     let mut exposed_ports = HashMap::new();
                    //     exposed_ports.insert("8080".to_string(), HashMap::new());
                    //     exposed_ports
                    // }),
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
                            "GAME_SERVER_PORT=28785".to_string(),
                            format!(
                                "SECRET_CHUNGUS={}",
                                std::env::var("SECRET_CHUNGUS").unwrap_or_default()
                            ),
                            format!(
                                "CHUNGUS_KEY={}",
                                std::env::var("CHUNGUS_KEY").unwrap_or_default()
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
}
