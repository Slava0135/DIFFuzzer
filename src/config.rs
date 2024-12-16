use serde::{Deserialize, Serialize};

use crate::abstract_fs::types::OperationWeights;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub operation_weights: OperationWeights,
}
