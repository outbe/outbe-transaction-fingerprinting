use anyhow::{anyhow, Error};
use halo2_axiom::arithmetic::Field;
use halo2_axiom::halo2curves::bn256::{Fr, G1};
use halo2_axiom::halo2curves::ff::PrimeField as PF;
use halo2_axiom::halo2curves::group::Group;
use halo2_axiom::halo2curves::CurveExt;

use std::marker::PhantomData;

use futures::future::ready;
use futures::{StreamExt, TryFutureExt};

use crate::protocols::FingerprintProtocol;
use crate::{Compact, HashSqueeze, HASH_TO_CURVE_PREFIX};

use crate::secret_sharing::SecretSharing;
use rand_core::OsRng;

pub trait AgentsTopology<F: PF, G: Group<Scalar = F>> {
    ///
    /// Returns how many of agents in the network
    fn count(&self) -> usize;

    ///
    /// Returns what the threshold for lagrange interpolation
    fn threshold(&self) -> usize;

    fn compute_coefficient(&self, agent: usize, cooperative_agents: &[usize]) -> F {
        SecretSharing::lagrange_coefficient(agent, cooperative_agents)
    }

    ///
    /// Send request and wait for response from the remote `agent`
    fn obtain_shard(
        &self,
        agent: usize,
        generation: u64,
        blinded_value: G,
    ) -> impl ::std::future::Future<Output = Result<(usize, G), Error>> + Send;
}

pub struct CollaborativeProtocol<F: PF, G: Group<Scalar = F>, T: AgentsTopology<F, G>> {
    agent: usize,    // agent number
    secret_shard: F, // our own secret shard
    topology: T,
    _phantom: PhantomData<G>,
}

impl<F: PF, G: Group<Scalar = F>, T: AgentsTopology<F, G>> CollaborativeProtocol<F, G, T> {
    pub fn new(agent_info: (usize, F), topology: T) -> Self {
        Self {
            agent: agent_info.0,
            secret_shard: agent_info.1,
            topology,
            _phantom: Default::default(),
        }
    }
}

impl<T: AgentsTopology<Fr, G1> + Sync> FingerprintProtocol<Fr>
    for CollaborativeProtocol<Fr, G1, T>
{
    async fn process(&self, unblinded: Fr) -> Result<Fr, Error> {
        let mut rng = OsRng::default();

        log::debug!("Processing unblinded value: {}", unblinded.compact());

        let curve_point = {
            // Reflect unblinded Fr on curve via hash_to_curve Eligator2 function
            let hasher = G1::hash_to_curve(HASH_TO_CURVE_PREFIX);
            hasher(&unblinded.to_bytes())
        };

        // Select the blinding factor `r`
        let blinding_factor = Fr::random(&mut rng);

        // Compute the blinded_hash
        let blinded_hash = curve_point * blinding_factor;

        // Collect the threshold responses from agents
        let mut responses = futures::stream::iter(1..=self.topology.count())
            .filter(|agent| ready(agent.clone() != self.agent))
            .map(|i| {
                let agent = i.clone();
                self.topology
                    .obtain_shard(i, 0, blinded_hash.clone())
                    .map_err(move |e| {
                        log::error!("Error while obtaining shard from agent {}: {}", agent, e);
                        e
                    })
                    .map_ok_or_else(|_| (0, G1::generator()), |v| v) // Todo add logging here
            })
            .buffer_unordered(1024) // TODO parametrize concurrency
            .filter(|(p, _)| ready(p.clone() > 0))
            .take(self.topology.threshold() - 1) // Since we already have one response from self.agent
            .collect::<Vec<(usize, G1)>>()
            .await;

        responses.push((self.agent, blinded_hash * self.secret_shard));

        if responses.len() < self.topology.threshold() {
            return Err(anyhow!("Not enough responses from other agents"));
        }

        // Precompute cooperative agents indexes
        let indices = responses.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>();

        log::debug!(
            "Got {} results from other agents: {:?}",
            indices.len(),
            indices
        );

        let mut y: G1 = Default::default(); // zero point

        // Compute blinded version of [r * k] P
        for (i, e_i) in responses {
            let lambda_i = self.topology.compute_coefficient(i, &indices);

            y += e_i * lambda_i;
        }

        // Unblind
        let unblinding_factor = blinding_factor.invert().unwrap();
        let hash_with_secret = y * unblinding_factor; // This is [k] P

        let fingerprint = hash_with_secret.squeeze();

        if log::log_enabled!(log::Level::Debug) {
            match &fingerprint {
                Ok(ref fp) => {
                    log::debug!("Computed fingerprint: {}", fp.compact());
                }
                Err(ref e) => {
                    log::error!("Error while computing fingerprint: {}", e);
                }
            }
        }

        fingerprint
    }
}
