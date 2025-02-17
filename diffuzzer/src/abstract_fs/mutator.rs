/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::HashSet;

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{
    fs::AbstractFS,
    generator::append_one,
    operation::{Operation, OperationWeights},
    workload::Workload,
};

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum MutationKind {
    Insert,
    Remove,
}

/// Weights determine the likelihood of mutation to be picked.
#[derive(Serialize, Deserialize, Clone)]
pub struct MutationWeights {
    pub weights: Vec<(MutationKind, u32)>,
}

#[allow(dead_code)]
impl MutationWeights {
    pub fn new(weights: Vec<(MutationKind, u32)>) -> Self {
        Self { weights }
    }
}

/// Tries to remove operation from workload at the index.
pub fn remove(workload: &Workload, index: usize) -> Option<Workload> {
    let mut ops = workload.ops.clone();
    ops.remove(index);
    let mut fs = AbstractFS::new();
    if fs.replay(&Workload { ops }).is_err() {
        None
    } else {
        Some(fs.recording)
    }
}

/// Tries to insert random operation to workload at the index.
pub fn insert(
    rng: &mut impl Rng,
    workload: &Workload,
    index: usize,
    weights: &OperationWeights,
) -> Option<Workload> {
    let mut used_names = HashSet::new();
    for op in workload.ops.iter() {
        match op {
            Operation::MkDir { path, mode: _ } => {
                for segment in path.segments() {
                    used_names.insert(segment);
                }
            }
            Operation::Create { path, mode: _ } => {
                for segment in path.segments() {
                    used_names.insert(segment);
                }
            }
            Operation::Remove { path: _ } => {}
            Operation::Hardlink { old_path, new_path } => {
                for segment in old_path.segments() {
                    used_names.insert(segment);
                }
                for segment in new_path.segments() {
                    used_names.insert(segment);
                }
            }
            Operation::Rename { old_path, new_path } => {
                for segment in old_path.segments() {
                    used_names.insert(segment);
                }
                for segment in new_path.segments() {
                    used_names.insert(segment);
                }
            }
            Operation::Open { path, des: _ } => {
                for segment in path.segments() {
                    used_names.insert(segment);
                }
            }
            Operation::Close { des: _ } => {}
            Operation::Read { des: _, size: _ } => {}
            Operation::Write {
                des: _,
                src_offset: _,
                size: _,
            } => {}
            Operation::FSync { des: _ } => {}
        }
    }

    let (before, after) = workload.ops.split_at(index);
    let mut fs = AbstractFS::new();
    if fs
        .replay(&Workload {
            ops: before.to_vec(),
        })
        .is_err()
    {
        return None;
    }

    let mut name_idx: usize = 0;
    let mut gen_name = || loop {
        let name = name_idx.to_string();
        name_idx += 1;
        if !used_names.contains(name.as_str()) {
            break name;
        }
    };
    append_one(rng, &mut fs, weights, &mut gen_name);
    if fs
        .replay(&Workload {
            ops: after.to_vec(),
        })
        .is_err()
    {
        None
    } else {
        Some(fs.recording)
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use crate::abstract_fs::{generator::generate_new, operation::OperationKind};

    use super::*;

    #[test]
    fn test_remove() {
        let w = Workload {
            ops: vec![
                Operation::MkDir {
                    path: "/foobar".into(),
                    mode: vec![],
                },
                Operation::Create {
                    path: "/foobar/boo".into(),
                    mode: vec![],
                },
                Operation::Create {
                    path: "/foobar/zoo".into(),
                    mode: vec![],
                },
            ],
        };
        assert_eq!(None, remove(&w, 0));
        assert_eq!(
            Some(Workload {
                ops: vec![
                    Operation::MkDir {
                        path: "/foobar".into(),
                        mode: vec![],
                    },
                    Operation::Create {
                        path: "/foobar/zoo".into(),
                        mode: vec![],
                    },
                ],
            }),
            remove(&w, 1)
        );
    }

    #[test]
    fn test_append() {
        let mut rng = StdRng::seed_from_u64(123);
        let w = Workload {
            ops: vec![
                Operation::MkDir {
                    path: "/foobar".into(),
                    mode: vec![],
                },
                Operation::Create {
                    path: "/foobar/boo".into(),
                    mode: vec![],
                },
                Operation::Remove {
                    path: "/foobar/boo".into(),
                },
            ],
        };
        assert_eq!(
            None,
            insert(
                &mut rng,
                &w,
                1,
                &OperationWeights::new(vec![(OperationKind::Remove, 100)])
            )
        );
        assert_eq!(
            Some(Workload {
                ops: vec![
                    Operation::MkDir {
                        path: "/foobar".into(),
                        mode: vec![],
                    },
                    Operation::Create {
                        path: "/foobar/boo".into(),
                        mode: vec![],
                    },
                    Operation::Remove {
                        path: "/foobar/boo".into(),
                    },
                    Operation::Remove {
                        path: "/foobar".into(),
                    },
                ],
            }),
            insert(
                &mut rng,
                &w,
                3,
                &OperationWeights::new(vec![(OperationKind::Remove, 100)])
            )
        );
    }

    #[test]
    fn smoke_test_mutate() {
        let mut rng = StdRng::seed_from_u64(123);
        let mut w = generate_new(&mut rng, 100, &OperationWeights::uniform());
        for _ in 0..1000 {
            let p: f64 = rng.gen();
            if w.ops.is_empty() || p >= 0.5 {
                let index = rng.gen_range(0..=w.ops.len());
                if let Some(workload) = insert(&mut rng, &w, index, &OperationWeights::uniform()) {
                    w = workload;
                }
            } else {
                let index = rng.gen_range(0..w.ops.len());
                if let Some(workload) = remove(&w, index) {
                    w = workload;
                }
            }
        }
    }
}
