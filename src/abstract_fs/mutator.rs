use std::collections::HashSet;

use rand::Rng;

use super::{
    generator::{append_one, OperationKind},
    types::{AbstractExecutor, Workload},
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
    pick_from: HashSet<OperationKind>,
) -> Option<Workload> {
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
    append_one(rng, &mut exec, pick_from);
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

mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use crate::abstract_fs::types::Operation;

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
            insert(&mut rng, &w, 1, HashSet::from([OperationKind::REMOVE]))
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
            insert(&mut rng, &w, 3, HashSet::from([OperationKind::REMOVE]))
        );
    }
}
