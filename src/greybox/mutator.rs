use log::debug;
use rand::{rngs::StdRng, seq::SliceRandom, Rng};

use crate::abstract_fs::{
    mutator::{insert, remove},
    types::{MutationKind, MutationWeights, OperationWeights, Workload},
};

pub struct Mutator {
    pub rng: StdRng,
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
    pub max_length: u16,
}

impl Mutator {
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

impl Mutator {
    fn mutate(&mut self, input: &mut Workload) -> bool {
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
                    true
                } else {
                    false
                }
            }
            MutationKind::REMOVE => {
                let index = self.rng.gen_range(0..input.ops.len());
                if let Some(workload) = remove(&input, index) {
                    *input = workload;
                    true
                } else {
                    false
                }
            }
        }
    }
}
