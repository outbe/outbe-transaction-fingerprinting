use anyhow::Result;
use clap::Parser;
use fingerprinting_core::secret_sharing::SecretSharing;
use fingerprinting_core::Compact;
use halo2_axiom::arithmetic::Field;
use halo2_axiom::halo2curves::bn256::Fr;
use rand_core::OsRng;

/// Generate a transaction fingerprint
#[derive(Parser, Debug)]
#[command(name = "fingerprinting-cli")]
#[command(about = "Fingerprint CLI utility", long_about = None)]
struct Args {
    /// Threshold for cooperative computation
    #[arg(long)]
    threshold: usize,

    /// Total number of cooperative agents network size
    #[arg(long)]
    agents: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut rng = OsRng;

    let random_secret = Fr::random(&mut rng);

    let secret_sharing = SecretSharing::generate(random_secret, args.threshold, args.agents);

    let shares_set = secret_sharing.get_shares();

    println!("Random secret: {}", random_secret.compact());
    println!("Shares:");
    for (agent, secret) in shares_set.iter() {
        println!("== share {}: {}", agent, secret.compact());
    }

    Ok(())
}
