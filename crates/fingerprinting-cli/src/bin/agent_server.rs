use clap::Parser;
use fingerprinting_cli::config::{FingerprintServiceConfig, GrpcConfig};
use fingerprinting_core::{CollaborativeProtocol, Compact, NaiveProtocol};
use fingerprinting_grpc::{net as fp, FingerprintService};
use fingerprinting_grpc_agent::{net as fp_agent, CooperationAgentService, GrpcAgentsTopology};
use halo2_axiom::halo2curves::bn256::Fr;
use hocon::HoconLoader;
use serde_derive::Deserialize;
use std::net::SocketAddr;
use volo_grpc::codegen::futures;
use volo_grpc::server::{Server, ServiceBuilder};

#[derive(Parser, Debug)]
#[command(name = "fingerprinting-agent")]
#[command(about = "Fingerprint Agent", long_about = None)]
struct Args {
    /// Config file location
    #[arg(long)]
    config: String,
}

#[derive(Deserialize)]
struct FingerprintingServiceConfig {
    grpc: GrpcConfig,
    #[serde(rename = "agent-grpc")]
    agent_grpc: GrpcConfig,
    #[serde(rename = "fingerprint-service")]
    fingerprint_service: FingerprintServiceConfig,
}
#[volo::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting fingerprinting agent...");

    let args = Args::parse();
    let reference_config = include_str!("../../config/agent-reference.conf");
    log::info!("== loading configuration from {}", args.config);

    let conf: FingerprintingServiceConfig = HoconLoader::new()
        .load_str(reference_config)?
        .load_file(args.config)?
        .resolve()?;

    let (fingerprint_server, agent_server): (Server, Option<Server>) = match conf
        .fingerprint_service
    {
        FingerprintServiceConfig::Cooperative(topology_config) => {
            log::info!("== Starting CRA Fingerprint agent in Cooperative mode with {} agents and {} threshold", topology_config.agents, topology_config.threshold);
            let topology = GrpcAgentsTopology::new(
                topology_config.agents,
                topology_config.threshold,
                topology_config
                    .members
                    .iter()
                    .map(|agent| (agent.agent_id, agent.address.to_string()))
                    .collect(),
            );

            log::info!(
                "== Built topology with members: {:?}",
                topology_config.members
            );

            let current_agent_secret = Compact::unwrap(&topology_config.secret_shard)?;
            let cooperation_service = CooperationAgentService::new(current_agent_secret);

            let protocol = CollaborativeProtocol::new(
                (topology_config.agent_id, current_agent_secret),
                topology,
            );

            let fingerprint_server = Server::new().add_service(
                ServiceBuilder::new(fp::outbe::fingerprint::v1::FingerprintServiceServer::new(
                    FingerprintService::new(protocol),
                ))
                .build(),
            );

            let agent_server = Server::new().add_service(
                ServiceBuilder::new(
                    fp_agent::outbe::fingerprint::agent::v1::CooperationServiceServer::new(
                        cooperation_service,
                    ),
                )
                .build(),
            );

            (fingerprint_server, Some(agent_server))
        }
        FingerprintServiceConfig::Naive(naive) => {
            log::warn!(
                "== Starting CRA Fingerprint agent in Naive mode with predefined secret: {}",
                naive.secret
            );
            let secret: Fr = Compact::unwrap(&naive.secret)?;

            let protocol = NaiveProtocol::new(secret);

            (
                Server::new().add_service(
                    ServiceBuilder::new(fp::outbe::fingerprint::v1::FingerprintServiceServer::new(
                        FingerprintService::new(protocol),
                    ))
                    .build(),
                ),
                None,
            )
        }
    };

    let fingerprint_grpc_address = format!("{}:{}", conf.grpc.host, conf.grpc.port);

    log::info!(
        "== starting Fingerprint GRPC server on {}",
        fingerprint_grpc_address
    );
    let addr: SocketAddr = fingerprint_grpc_address.parse()?;

    let fingerprint_grpc_address = volo::net::Address::from(addr);

    match agent_server {
        None => fingerprint_server
            .http2_adaptive_window(true)
            .run(fingerprint_grpc_address)
            .await
            .map_err(|e| anyhow::anyhow!(e)),
        Some(agent_server) => {
            let agent_grpc_address = format!("{}:{}", conf.agent_grpc.host, conf.agent_grpc.port);

            log::info!("== starting Agent GRPC server on {}", agent_grpc_address);
            let addr: SocketAddr = agent_grpc_address.parse()?;

            let agent_grpc_address = volo::net::Address::from(addr);

            let agent_server = agent_server
                .http2_adaptive_window(true)
                .run(agent_grpc_address);

            let fingerprint_server = fingerprint_server
                .http2_adaptive_window(true)
                .run(fingerprint_grpc_address);

            futures::future::try_join(agent_server, fingerprint_server)
                .await
                .map(|_| ())
                .map_err(|e| anyhow::anyhow!(e))
        }
    }
}
