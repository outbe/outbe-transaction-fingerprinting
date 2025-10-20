mod collaborative_protocol;
mod naive_protocol;

use anyhow::Error;
use halo2_axiom::halo2curves::ff::PrimeField as PF;

pub use collaborative_protocol::AgentsTopology;
pub use collaborative_protocol::CollaborativeProtocol;
pub use naive_protocol::NaiveProtocol;

pub trait FingerprintProtocol<F: PF> {
    fn process(&self, unblinded: F) -> impl ::std::future::Future<Output = Result<F, Error>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    use halo2_axiom::halo2curves::bn256::{Fr, G1};
    use halo2_axiom::halo2curves::ff::Field;
    use rand_core::OsRng;

    use crate::secret_sharing::SecretSharing;

    use crate::protocols::AgentsTopology;
    use crate::protocols::CollaborativeProtocol;
    use crate::protocols::NaiveProtocol;

    struct LocalAgentsTopology {
        sss: SecretSharing<Fr>,
    }

    impl AgentsTopology<Fr, G1> for LocalAgentsTopology {
        fn count(&self) -> usize {
            10
        }

        fn threshold(&self) -> usize {
            self.sss.threshold
        }

        fn compute_coefficient(&self, agent: usize, cooperative_agents: &[usize]) -> Fr {
            SecretSharing::lagrange_coefficient(agent, cooperative_agents)
        }

        async fn obtain_shard(
            &self,
            agent: usize,
            _: u64,
            blinded_value: G1,
        ) -> Result<(usize, G1), Error> {
            Ok(self.sss.compute_exponent(agent, blinded_value))
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cooperative_fingerprint_protocol() -> Result<(), Error> {
        let mut rng = OsRng;
        let secret = Fr::random(&mut rng);
        let sss = SecretSharing::generate(secret, 6, 10);

        let origin = Fr::from(42u64);

        // We are the 1st agent
        let current_share = sss.get_share(1).unwrap();

        let topology = LocalAgentsTopology { sss };

        let coop_protocol = CollaborativeProtocol::new((1, current_share), topology);
        let naive_protocol = NaiveProtocol::new(secret);

        let processed = coop_protocol.process(origin).await?;
        let naive_processed = naive_protocol.process(origin).await?;

        println!("processed: {:?}", processed);
        println!("naive_processed: {:?}", naive_processed);

        assert_eq!(processed, naive_processed);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fingerprint_protocol() -> Result<(), Error> {
        let mut rng = OsRng;

        let secret = Fr::random(&mut rng);
        let origin = Fr::from(42u64);

        let fingerprint_protocol = NaiveProtocol::new(secret);

        let processed = fingerprint_protocol.process(origin).await?;

        println!("processed: {:?}", processed);

        Ok(())
    }
}
