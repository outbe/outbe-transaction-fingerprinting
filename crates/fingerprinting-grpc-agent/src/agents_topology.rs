use crate::net::outbe::fingerprint::agent::v1::{CooperationRequest, CooperationServiceClient};
use anyhow::Error;
use fingerprinting_core::AgentsTopology;
use halo2_axiom::halo2curves::bn256::{Fr, G1Compressed, G1};
use halo2_axiom::halo2curves::group::GroupEncoding;
use pilota::Bytes;
use rand::Rng;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use volo::net::Address;

pub struct GrpcAgentsTopology {
    count: usize,
    threshold: usize,
    members: HashMap<usize, Vec<CooperationServiceClient>>,
}

impl GrpcAgentsTopology {
    pub fn new(count: usize, threshold: usize, members: Vec<(usize, String)>) -> Self {
        let members: HashMap<usize, Vec<CooperationServiceClient>> = members
            .iter()
            .map(|(position, addr)| {
                let clients_for_addr = GrpcAgentsTopology::build_client(addr).unwrap_or_default();

                (position.clone(), clients_for_addr)
            })
            .collect();

        Self {
            count,
            threshold,
            members,
        }
    }

    fn build_client(
        remote_address: &String,
    ) -> Result<Vec<CooperationServiceClient>, anyhow::Error> {
        let clients = remote_address
            .to_socket_addrs()?
            .map(|address| GrpcAgentsTopology::get_client(address))
            .collect::<Vec<_>>();

        Ok(clients)
    }

    fn get_client(addr: SocketAddr) -> CooperationServiceClient {
        crate::net::outbe::fingerprint::agent::v1::CooperationServiceClientBuilder::new(format!(
            "inter-agent-coop-service-{}",
            addr
        ))
        .address(Address::from(addr))
        .build()
    }
}

impl AgentsTopology<Fr, G1> for GrpcAgentsTopology {
    fn count(&self) -> usize {
        self.count
    }

    fn threshold(&self) -> usize {
        self.threshold
    }

    async fn obtain_shard(&self, agent: usize, generation: u64, blinded_value: G1) -> Result<(usize, G1), Error> {
        if agent == 0 || agent > self.count {
            return Err(anyhow::anyhow!(
                "Invalid agent number, should be in range 1 to {}",
                self.count
            ));
        }

        let clients = self
            .members
            .get(&agent)
            .ok_or(anyhow::anyhow!("No clients for agent {}", agent))?;
        let client = rand::thread_rng().gen_range(0..clients.len());
        let client = &clients[client];

        let bytes = blinded_value.to_bytes();

        let exponent = client
            .compute_exponent(CooperationRequest {
                generation,
                blinded_value: Bytes::copy_from_slice(bytes.as_ref()),
                _unknown_fields: Default::default(),
            })
            .await?;

        let exponent = exponent.into_inner().blinded_exponent;
        let mut exponent_point = G1Compressed::default();

        // todo verify that received bytes are exactly 32 bytes
        exponent_point.as_mut().copy_from_slice(exponent.as_ref());
        let exponent_point =
            G1::from_bytes(&exponent_point)
                .into_option()
                .ok_or(anyhow::anyhow!(
                    "Invalid exponent point, agent {} returned wrong value",
                    agent
                ))?;

        Ok((agent, exponent_point))
    }
}
