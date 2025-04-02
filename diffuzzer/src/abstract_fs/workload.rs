/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use base64::{Engine, prelude::BASE64_URL_SAFE};
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
    pub fn cut(&mut self, index: u32) {
        self.ops.truncate(index as usize + 1);
    }
}
