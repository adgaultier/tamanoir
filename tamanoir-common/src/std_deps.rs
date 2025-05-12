use std::{fmt, str::FromStr};

use crate::TargetArch;

#[derive(Debug, Clone, Copy)]
pub enum Layout {
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

impl Layout {
    pub const ALL: [Self; 2] = [Self::Qwerty, Self::Azerty];
}

impl std::fmt::Display for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Layout::Qwerty => write!(f, "qwerty"),
            Layout::Azerty => write!(f, "azerty"),
            Layout::Unknown => write!(f, "?"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Engine {
    Docker,
    Podman,
}
impl TargetArch {
    pub const ALL: [TargetArch; 2] = [TargetArch::X86_64, TargetArch::Aarch64];
}
impl fmt::Display for TargetArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetArch::X86_64 => write!(f, "x86_64"),
            TargetArch::Aarch64 => write!(f, "aarch64"),
            TargetArch::Unknown => write!(f, "?"),
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
            _ => Err(format!("{s} engine isn't implmented")),
        }
    }
}

impl FromStr for TargetArch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86_64" => Ok(TargetArch::X86_64),
            "aarch64" => Ok(TargetArch::Aarch64),
            _ => Err(format!("{s} arch isn't implmented")),
        }
    }
}

impl TryFrom<u8> for TargetArch {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::X86_64),
            1 => Ok(Self::Aarch64),
            _ => Err(format!("{value} arch enum not recognized ")),
        }
    }
}
