mod grain;
mod matrix;
mod permutation;
mod poseidon;
mod spec;

pub(crate) mod ff {
    // Simple re-export types for simplify imports
    pub(crate) use halo2_axiom::halo2curves::group::ff::{FromUniformBytes, PrimeField};
}

pub use crate::poseidon::Poseidon;
pub use crate::spec::{MDSMatrices, MDSMatrix, SparseMDSMatrix, Spec, State};
