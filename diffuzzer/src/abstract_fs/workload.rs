use base64::{prelude::BASE64_URL_SAFE, Engine};
use serde::{Deserialize, Serialize};
use siphasher::sip128::SipHasher13;

use super::operation::Operation;

/// Sequence of operations to be run in test. 
#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
pub struct Workload {
    pub ops: Vec<Operation>,
}

impl Workload {
    pub fn new() -> Workload {
        Workload { ops: vec![] }
    }
    pub fn push(&mut self, op: Operation) {
        self.ops.push(op);
    }
    pub fn generate_name(&self) -> String {
        let bytes = bincode::serialize(self).unwrap();
        let hasher = SipHasher13::new();
        let hash = hasher.hash(&bytes).as_bytes();
        BASE64_URL_SAFE.encode(hash)
    }
}
