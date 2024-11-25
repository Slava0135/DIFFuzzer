use std::{
    borrow::Cow,
    collections::HashSet,
    hash::{DefaultHasher, Hash, Hasher},
    vec,
};

use libafl::{
    corpus::CorpusId,
    generators::Generator,
    inputs::Input,
    mutators::{MutationResult, Mutator},
    state::HasRand,
    Error,
};
use libafl_bolts::Named;
use rand::{rngs::StdRng, seq::index, Rng};

use crate::abstract_fs::{
    generator::{generate_new, OperationKind},
    mutator::{insert, remove},
    types::Workload,
};

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

impl<S> Generator<Workload, S> for WorkloadGenerator {
    fn generate(&mut self, _state: &mut S) -> Result<Workload, Error> {
        let size = self.rng.gen_range(1..=self.max_size);
        Ok(generate_new(&mut self.rng, size))
    }
}

pub struct WorkloadMutator {
    pub rng: StdRng,
}

impl<S> Mutator<Workload, S> for WorkloadMutator
where
    S: HasRand,
{
    fn mutate(&mut self, _state: &mut S, input: &mut Workload) -> Result<MutationResult, Error> {
        let p: f64 = self.rng.gen();
        if p > 0.3 {
            let index = self.rng.gen_range(0..=input.ops.len());
            if let Some(workload) = insert(&mut self.rng, &input, index, OperationKind::all()) {
                *input = workload;
                Ok(MutationResult::Mutated)
            } else {
                Ok(MutationResult::Skipped)
            }
        } else {
            let index = self.rng.gen_range(0..input.ops.len());
            if let Some(workload) = remove(&input, index) {
                *input = workload;
                Ok(MutationResult::Mutated)
            } else {
                Ok(MutationResult::Skipped)
            }
        }
    }
}

impl Named for WorkloadMutator {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("WorkloadMutator")
    }
}
