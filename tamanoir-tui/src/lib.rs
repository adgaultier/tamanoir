pub mod app;

pub mod event;

pub mod tui;

pub mod handler;

pub mod grpc;

pub mod section;

pub mod tamanoir_grpc {
    tonic::include_proto!("tamanoir");
}
