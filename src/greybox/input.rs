use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
};

use libafl::{
    Error,
    corpus::CorpusId,
    inputs::Input,
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::Named;
use log::debug;
use rand::{Rng, rngs::StdRng, seq::SliceRandom};

use crate::abstract_fs::{
    mutator::{insert, remove},
    types::{MutationKind, MutationWeights, OperationWeights, Workload},
};

impl Input for Workload {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

pub struct WorkloadMutator {
    pub rng: StdRng,
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
    pub max_length: u16,
}

impl WorkloadMutator {
    pub fn new(
        rng: StdRng,
        operation_weights: OperationWeights,
        mutation_weights: MutationWeights,
        max_length: u16,
    ) -> Self {
        Self {
            rng,
            operation_weights,
            mutation_weights,
            max_length,
        }
    }
}

impl<S> Mutator<Workload, S> for WorkloadMutator
where
    S: HasRand,
{
    fn mutate(&mut self, _state: &mut S, input: &mut Workload) -> Result<MutationResult, Error> {
        debug!("mutating input");
        let mut mutations = self.mutation_weights.clone();
        if input.ops.is_empty() {
            mutations
                .weights
                .retain(|(op, _)| *op != MutationKind::REMOVE);
        }
        if input.ops.len() >= self.max_length.into() {
            mutations
                .weights
                .retain(|(op, _)| *op != MutationKind::INSERT);
        }
        match mutations
            .weights
            .choose_weighted(&mut self.rng, |item| item.1)
            .unwrap()
            .0
        {
            MutationKind::INSERT => {
                let index = self.rng.gen_range(0..=input.ops.len());
                if let Some(workload) =
                    insert(&mut self.rng, &input, index, &self.operation_weights)
                {
                    *input = workload;
                    Ok(MutationResult::Mutated)
                } else {
                    Ok(MutationResult::Skipped)
                }
            }
            MutationKind::REMOVE => {
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
}

impl Named for WorkloadMutator {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("WorkloadMutator")
    }
}
