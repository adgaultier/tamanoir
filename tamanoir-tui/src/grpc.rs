use std::{collections::HashMap, net::Ipv4Addr};

use tonic::transport::Channel;

use crate::{
    app::AppResult,
    session::utils::init_keymaps,
    tamanoir_grpc::{session_client::SessionClient, Empty, SessionResponse},
};

type SessionsMap = HashMap<String, SessionResponse>;
#[derive(Debug)]
pub struct Grpc {
    pub session_client: SessionClient<Channel>,
    pub sessions: SessionsMap,
}

impl Grpc {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        let mut session_client: SessionClient<Channel> =
            SessionClient::connect(format!("http://{}:{}", ip, port)).await?;
        init_keymaps();
        let request = tonic::Request::new(Empty {});
        let mut sessions_map: SessionsMap = HashMap::new();
        let sessions = session_client
            .list_sessions(request)
            .await?
            .into_inner()
            .sessions;
        for s in sessions.into_iter() {
            sessions_map.insert(s.ip.clone(), s);
        }
        Ok(Self {
            session_client,
            sessions: sessions_map,
        })
    }
}
