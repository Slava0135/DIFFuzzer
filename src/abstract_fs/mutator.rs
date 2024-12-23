use std::collections::HashSet;

use rand::Rng;

use super::{
    generator::append_one,
    types::{AbstractExecutor, Operation, OperationWeights, Workload},
};

pub fn remove(workload: &Workload, index: usize) -> Option<Workload> {
    let mut ops = workload.ops.clone();
    ops.remove(index);
    let mut exec = AbstractExecutor::new();
    if !exec.replay(&Workload { ops }).is_ok() {
        None
    } else {
        Some(exec.recording)
    }
}

pub fn insert(
    rng: &mut impl Rng,
    workload: &Workload,
    index: usize,
    weights: &OperationWeights,
) -> Option<Workload> {
    let mut used_names = HashSet::new();
    for op in workload.ops.iter() {
        match op {
            Operation::MKDIR { path, mode: _ } => {
                for segment in path.split("/") {
                    used_names.insert(segment);
                }
            }
            Operation::CREATE { path, mode: _ } => {
                for segment in path.split("/") {
                    used_names.insert(segment);
                }
            }
            Operation::REMOVE { path: _ } => {}
        }
    }

    let (before, after) = workload.ops.split_at(index);
    let mut exec = AbstractExecutor::new();
    if !exec
        .replay(&Workload {
            ops: before.to_vec(),
        })
        .is_ok()
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
    append_one(rng, &mut exec, weights, &mut gen_name);
    if !exec
        .replay(&Workload {
            ops: after.to_vec(),
        })
        .is_ok()
    {
        None
    } else {
        Some(exec.recording)
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use crate::abstract_fs::{
        generator::generate_new,
        types::{Operation, OperationKind},
    };

    use super::*;

    #[test]
    fn test_remove() {
        let w = Workload {
            ops: vec![
                Operation::MKDIR {
                    path: "/foobar".to_owned(),
                    mode: vec![],
                },
                Operation::CREATE {
                    path: "/foobar/boo".to_owned(),
                    mode: vec![],
                },
                Operation::CREATE {
                    path: "/foobar/zoo".to_owned(),
                    mode: vec![],
                },
            ],
        };
        assert_eq!(None, remove(&w, 0));
        assert_eq!(
            Some(Workload {
                ops: vec![
                    Operation::MKDIR {
                        path: "/foobar".to_owned(),
                        mode: vec![],
                    },
                    Operation::CREATE {
                        path: "/foobar/zoo".to_owned(),
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
                Operation::MKDIR {
                    path: "/foobar".to_owned(),
                    mode: vec![],
                },
                Operation::CREATE {
                    path: "/foobar/boo".to_owned(),
                    mode: vec![],
                },
                Operation::REMOVE {
                    path: "/foobar/boo".to_owned(),
                },
            ],
        };
        assert_eq!(
            None,
            insert(
                &mut rng,
                &w,
                1,
                &OperationWeights::new(vec![(OperationKind::REMOVE, 100)])
            )
        );
        assert_eq!(
            Some(Workload {
                ops: vec![
                    Operation::MKDIR {
                        path: "/foobar".to_owned(),
                        mode: vec![],
                    },
                    Operation::CREATE {
                        path: "/foobar/boo".to_owned(),
                        mode: vec![],
                    },
                    Operation::REMOVE {
                        path: "/foobar/boo".to_owned(),
                    },
                    Operation::REMOVE {
                        path: "/foobar".to_owned(),
                    },
                ],
            }),
            insert(
                &mut rng,
                &w,
                3,
                &OperationWeights::new(vec![(OperationKind::REMOVE, 100)])
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
