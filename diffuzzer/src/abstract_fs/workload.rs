/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use base64::{Engine, prelude::BASE64_URL_SAFE};
use serde::{Deserialize, Serialize};
use twox_hash::XxHash3_128;

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
        let mut hasher = XxHash3_128::new();
        hasher.write(&bytes);
        let hash = hasher.finish_128();
        BASE64_URL_SAFE.encode(hash.to_le_bytes())
    }
}
