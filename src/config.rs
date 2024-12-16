use serde::{Deserialize, Serialize};

use crate::abstract_fs::types::{MutationWeights, OperationWeights};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
    pub max_workload_length: u16,
}
