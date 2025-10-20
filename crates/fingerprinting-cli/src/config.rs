use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AgentConfig {
    pub agent_id: usize,
    pub secret_shard: String,
}
#[derive(Deserialize, Debug)]
pub struct AgentReferenceConfig {
    pub agent_id: usize,
    pub address: String,
}

#[derive(Deserialize, Debug)]
pub struct GrpcConfig {
    pub host: String,
    pub port: u16,
}
#[derive(Deserialize, Debug)]
pub struct CooperativeTopologyConfig {
    pub agent_id: usize,
    pub secret_shard: String,
    pub agents: usize,
    pub threshold: usize,
    pub members: Vec<AgentReferenceConfig>,
}

#[derive(Deserialize, Debug)]
pub struct NaiveTopologyConfig {
    pub secret: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum FingerprintServiceConfig {
    Cooperative(CooperativeTopologyConfig),
    Naive(NaiveTopologyConfig),
}
