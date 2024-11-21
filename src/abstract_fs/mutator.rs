use super::types::{AbstractExecutor, Workload};

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

mod tests {
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
}
