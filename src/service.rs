pub mod chungustrator_rpc {
    tonic::include_proto!("chungustrator");
}

use crate::orchestrator::{OrchestratorMessage, OrchestratorResponse};
use crate::service::chungustrator_rpc::{MatchRequest, MatchResponse};
use chungustrator_rpc::chungustrator_server::Chungustrator;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct ChungustratorService {
    pub tx: mpsc::UnboundedSender<OrchestratorMessage>,
}

#[tonic::async_trait]
impl Chungustrator for ChungustratorService {
    async fn create_match(
        &self,
        request: Request<MatchRequest>,
    ) -> Result<Response<MatchResponse>, Status> {
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let verification_codes = request.into_inner().verification_codes;

        self.tx
            .send(OrchestratorMessage::CreateContainer {
                verification_codes,
                response_tx,
            })
            .map_err(|e| Status::internal(format!("Something happened: {}", e)))?;

        match response_rx.recv().await {
            Some(OrchestratorResponse::ContainerCreationSuccess {
                id,
                wan_ip,
                lan_ip,
                port,
            }) => Ok(Response::new(MatchResponse {
                id,
                ip_address: wan_ip,
                lan_address: lan_ip,
                port: port as u32,
            })),
            Some(OrchestratorResponse::ContainerCreationError { error }) => {
                Err(Status::internal(error))
            }
            None => Err(Status::internal("No response received")),
        }
    }
}
