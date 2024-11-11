use std::collections::HashSet;

use rand::{seq::{IteratorRandom, SliceRandom}, Rng};

use crate::abstract_fs::{self, AbstractExecutor};

enum Operation {
    MKDIR,
    CREATE,
    REMOVE,
}

pub fn generate_new(rng: &mut impl Rng, size: usize) -> Vec<abstract_fs::Operation> {
    let mut executor = AbstractExecutor::new();
    let mut name_idx = 1;
    for _ in 0..size {
        let alive = executor.alive();
        let alive_dirs: Vec<abstract_fs::DirIndex> = alive
            .iter()
            .filter_map(|n| match n {
                abstract_fs::Node::DIR(dir) => Some(dir.clone()),
                abstract_fs::Node::FILE(_) => None,
            })
            .collect();
        match [Operation::MKDIR, Operation::CREATE, Operation::REMOVE]
            .choose(rng)
            .unwrap()
        {
            Operation::MKDIR => {
                executor.mkdir(
                    alive_dirs.choose(rng).unwrap(),
                    name_idx.to_string(),
                    HashSet::new(),
                );
                name_idx += 1;
            }
            Operation::CREATE => {
                executor.create(
                    alive_dirs.choose(rng).unwrap(),
                    name_idx.to_string(),
                    HashSet::new(),
                );
                name_idx += 1;
            }
            Operation::REMOVE => {
                let node = alive.iter().filter(|n| match n {
                    abstract_fs::Node::FILE(_) => true,
                    abstract_fs::Node::DIR(dir) => *dir != AbstractExecutor::root_index(),
                }).choose(rng).unwrap();
                executor.remove(node);
            }
        }
    }
    executor.recording
}
