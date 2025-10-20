mod agents_topology;

// hide generated values in private module
mod generator {
    include!(concat!(env!("OUT_DIR"), "/proto_gen.rs"));
}
pub use agents_topology::GrpcAgentsTopology;
pub use generator::proto_gen::*;

use halo2_axiom::halo2curves::bn256::{Fr, G1Compressed, G1};
use halo2_axiom::halo2curves::group::GroupEncoding;
use pilota::Bytes;
use volo_grpc::{Code, Request, Response, Status};

use net::outbe::fingerprint::agent::v1::{CooperationRequest, CooperationResponse};

pub struct CooperationAgentService {
    agent_secret_shard: Fr,
}

impl CooperationAgentService {
    pub fn new(secret_shard: Fr) -> CooperationAgentService {
        CooperationAgentService {
            agent_secret_shard: secret_shard,
        }
    }
}

impl net::outbe::fingerprint::agent::v1::CooperationService for CooperationAgentService {
    async fn compute_exponent(
        &self,
        req: Request<CooperationRequest>,
    ) -> Result<Response<CooperationResponse>, Status> {
        let request = req.into_inner();
        let blinded_value = request.blinded_value;
        let generation = request.generation;

        if generation != 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                "Current implementation doesn't support secret generations",
            ));
        }

        if blinded_value.len() != 32 {
            return Err(Status::new(
                Code::InvalidArgument,
                "Invalid blinded value, it should be exactly 32 bytes long",
            ));
        }
        let mut point = G1Compressed::default();
        point.as_mut().copy_from_slice(blinded_value.as_ref());

        let b_point = G1::from_bytes(&point).into_option().ok_or(Status::new(
            Code::InvalidArgument,
            "Invalid blinded value, it should be a valid G1 point",
        ))?;

        let exponent = b_point * self.agent_secret_shard;
        let exponent_bytes = exponent.to_bytes();

        let response = CooperationResponse {
            generation,
            blinded_exponent: Bytes::copy_from_slice(exponent_bytes.as_ref()),
            proof_of_computation: Default::default(),
            _unknown_fields: Default::default(),
        };

        Ok(Response::new(response))
    }
}
