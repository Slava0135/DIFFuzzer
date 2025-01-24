use serde::{Deserialize, Serialize};

use crate::abstract_fs::{mutator::MutationWeights, operation::OperationWeights};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub greybox: GreyboxConfig,
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
    pub max_workload_length: u16,
    pub fs_name: String,
    pub hashing_enabled: bool,
    pub heartbeat_interval: u16,
    pub timeout: u8,
}

#[derive(Serialize, Deserialize)]
pub struct GreyboxConfig {
    pub max_mutations: u16,
}
