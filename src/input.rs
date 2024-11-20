use std::hash::{DefaultHasher, Hash, Hasher};

use libafl::{corpus::CorpusId, inputs::Input};

use crate::abstract_fs::Workload;

impl Input for Workload {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
