use std::{collections::HashMap, net::Ipv4Addr};

use tamanoir_grpc::{session_client::SessionClient, Empty};
use tonic::transport::Channel;

use crate::{app::AppResult, grpc::tamanoir_grpc::SessionResponse};

pub mod tamanoir_grpc {
    tonic::include_proto!("tamanoir");
}

#[derive(Debug)]
pub struct Grpc {
    pub session_client: SessionClient<Channel>,
    pub sessions: HashMap<Ipv4Addr, Vec<u8>>,
}

impl Grpc {
    pub async fn new(ip: Ipv4Addr, port: u16) -> AppResult<Self> {
        let session_client: SessionClient<Channel> =
            SessionClient::connect(format!("http://{}:{}", ip, port)).await?;

        // Initial hashmap

        Ok(Self {
            session_client,
            sessions: HashMap::new(),
        })
    }

    pub async fn get_sessions(&mut self) -> AppResult<HashMap<Ipv4Addr, Vec<u8>>> {
        let request = tonic::Request::new(Empty {});
        let response = self
            .session_client
            .list_sessions(request)
            .await?
            .into_inner();
        dbg!(response.sessions);

        Ok(HashMap::new())
    }
}
