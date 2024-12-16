use serde::{Deserialize, Serialize};

use crate::abstract_fs::types::{MutationWeights, OperationWeights};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub greybox: GreyboxConfig,
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
}

#[derive(Serialize, Deserialize)]
pub struct GreyboxConfig {
    pub max_workload_length: u16,
    pub max_mutations: u16,
    pub timeout: u8,
}
