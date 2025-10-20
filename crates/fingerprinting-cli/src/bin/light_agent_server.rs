use clap::Parser;
use fingerprinting_grpc_agent::{net, CooperationAgentService};
use halo2_axiom::halo2curves::bn256::Fr;
use hocon::HoconLoader;
use serde_derive::Deserialize;
use std::net::SocketAddr;
use volo_grpc::server::{Server, ServiceBuilder};

use fingerprinting_cli::config::{AgentConfig, GrpcConfig};
use fingerprinting_core::Compact;

#[derive(Parser, Debug)]
#[command(name = "fingerprinting-light-agent")]
#[command(about = "Fingerprint Light Agent", long_about = None)]
struct Args {
    /// Config file location
    #[arg(long)]
    config: String,
}

#[derive(Deserialize)]
struct LightAgentConfig {
    grpc: GrpcConfig,
    agent: AgentConfig,
}

#[volo::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting fingerprinting light agent...");

    let args = Args::parse();
    let reference_config = include_str!("../../config/light-agent-reference.conf");
    log::info!("== loading configuration from {}", args.config);
    let conf: LightAgentConfig = HoconLoader::new()
        .load_str(reference_config)?
        .load_file(args.config)?
        .resolve()?;

    let address = format!("{}:{}", conf.grpc.host, conf.grpc.port);

    log::info!("== starting GRPC server on {}", address);
    let addr: SocketAddr = address.parse()?;

    let addr = volo::net::Address::from(addr);
    let secret_shard: Fr =
        Compact::unwrap(&conf.agent.secret_shard).expect("Cannot parse secret shard");

    let service = CooperationAgentService::new(secret_shard);

    Server::new()
        .add_service(
            ServiceBuilder::new(
                net::outbe::fingerprint::agent::v1::CooperationServiceServer::new(service),
            )
            .build(),
        )
        .run(addr)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}
