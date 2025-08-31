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
        // let params = CreateContainerOptionsBuilder::new().build();

        let mut port_bindings = HashMap::<String, Option<Vec<PortBinding>>>::new();
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

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            ..Default::default()
        };

        let mut exposed_ports = HashMap::<String, HashMap<(), ()>>::new();
        exposed_ports.insert("27876/tcp".to_string(), HashMap::new());
        exposed_ports.insert("27876/udp".to_string(), HashMap::new());

        let config = ContainerCreateBody {
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            image: Some("chungusmod".to_string()),
            ..Default::default()
        };

        let container_id = self
            .client
            .create_container(
                None::<bollard::query_parameters::CreateContainerOptions>,
                config,
            )
            .await?
            .id;

        self.client
            .start_container(
                &container_id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await?;

        Ok(container_id)
    }
}
