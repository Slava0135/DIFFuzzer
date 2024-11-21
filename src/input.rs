use std::hash::{DefaultHasher, Hash, Hasher};

use libafl::{corpus::CorpusId, generators::Generator, inputs::Input, state::HasRand, Error};
use rand::{rngs::StdRng, Rng};

use crate::abstract_fs::{generator::generate_new, types::Workload};

impl Input for Workload {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

pub struct WorkloadGenerator {
    pub rng: StdRng,
    pub max_size: usize,
}

impl<S> Generator<Workload, S> for WorkloadGenerator
where
    S: HasRand,
{
    fn generate(&mut self, _state: &mut S) -> Result<Workload, Error> {
        let size = self.rng.gen_range(1..=self.max_size);
        Ok(generate_new(&mut self.rng, size))
    }
}
