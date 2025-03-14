pub mod app;

pub mod event;

pub mod tui;

pub mod grpc;

pub mod section;

pub mod notification;
pub mod tamanoir_grpc {
    tonic::include_proto!("tamanoir");
}
