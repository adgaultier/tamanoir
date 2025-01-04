pub mod app;

pub mod event;

pub mod ui;

pub mod tui;

pub mod handler;

pub mod notifications;

pub mod grpc;

pub mod session;

pub mod tamanoir_grpc {
    tonic::include_proto!("tamanoir");
}
