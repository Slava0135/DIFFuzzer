use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
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
use log::debug;
use rand::{rngs::StdRng, seq::SliceRandom, Rng};

use crate::abstract_fs::{
    generator::generate_new,
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

pub struct WorkloadGenerator {
    pub rng: StdRng,
    pub max_size: usize,
    pub weights: OperationWeights,
}

impl<S> Generator<Workload, S> for WorkloadGenerator {
    fn generate(&mut self, _state: &mut S) -> Result<Workload, Error> {
        let size = self.rng.gen_range(1..=self.max_size);
        Ok(generate_new(&mut self.rng, size, &self.weights))
    }
}

pub struct WorkloadMutator {
    pub rng: StdRng,
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
}

impl WorkloadMutator {
    pub fn new(
        rng: StdRng,
        operation_weights: OperationWeights,
        mutation_weights: MutationWeights,
    ) -> Self {
        Self {
            rng,
            operation_weights,
            mutation_weights,
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
