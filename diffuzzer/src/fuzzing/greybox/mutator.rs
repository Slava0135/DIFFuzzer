/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use rand::{Rng, rngs::StdRng, seq::SliceRandom};

use crate::abstract_fs::{
    mutator::{MutationKind, MutationWeights, insert, remove},
    operation::OperationWeights,
    workload::Workload,
};

pub struct Mutator {
    rng: StdRng,
    operation_weights: OperationWeights,
    mutation_weights: MutationWeights,
    max_length: u16,
    max_mutations: u16,
}

impl Mutator {
    pub fn new(
        rng: StdRng,
        operation_weights: OperationWeights,
        mutation_weights: MutationWeights,
        max_length: u16,
        max_mutations: u16,
    ) -> Self {
        Self {
            rng,
            operation_weights,
            mutation_weights,
            max_length,
            max_mutations,
        }
    }
}

impl Mutator {
    pub fn mutate(&mut self, input: Workload) -> Workload {
        let mut input = input;
        let mut count = 0;
        let n = self.rng.gen_range(1..=self.max_mutations);
        while count < n {
            if self.mutate_once(&mut input) {
                count += 1;
            }
        }
        input
    }
    fn mutate_once(&mut self, input: &mut Workload) -> bool {
        let mut mutations = self.mutation_weights.clone();
        if input.ops.is_empty() {
            mutations
                .weights
                .retain(|(op, _)| *op != MutationKind::Remove);
        }
        if input.ops.len() >= self.max_length.into() {
            mutations
                .weights
                .retain(|(op, _)| *op != MutationKind::Insert);
        }
        match mutations
            .weights
            .choose_weighted(&mut self.rng, |item| item.1)
            .unwrap()
            .0
        {
            MutationKind::Insert => {
                let index = self.rng.gen_range(0..=input.ops.len());
                if let Some(workload) = insert(&mut self.rng, input, index, &self.operation_weights)
                {
                    *input = workload;
                    true
                } else {
                    false
                }
            }
            MutationKind::Remove => {
                let index = self.rng.gen_range(0..input.ops.len());
                if let Some(workload) = remove(input, index) {
                    *input = workload;
                    true
                } else {
                    false
                }
            }
        }
    }
}
