pub mod chungustrator_rpc {
    tonic::include_proto!("chungustrator");
}

use crate::orchestrator::OrchestratorMessage;
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
        unimplemented!()
    }
}
