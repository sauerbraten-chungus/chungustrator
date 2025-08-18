use std::{collections::HashMap, default, sync::Arc};

use bollard::{
    Docker, models::ContainerCreateBody, query_parameters::CreateContainerOptionsBuilder,
};

pub struct Chungustrator {
    client: Docker,
    list: HashMap<String, String>,
}

impl Chungustrator {
    pub fn new() -> Result<Self, bollard::errors::Error> {
        let client = Docker::connect_with_local_defaults()?;

        Ok(Self {
            client,
            list: HashMap::new(),
        })
    }

    pub async fn create_container(&self) -> Result<String, bollard::errors::Error> {
        // let params = CreateContainerOptionsBuilder::new().build();

        let config = ContainerCreateBody {
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
