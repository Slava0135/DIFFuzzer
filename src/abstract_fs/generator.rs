use std::collections::HashSet;

use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};

use crate::abstract_fs::types::{AbstractExecutor, DirIndex, ModeFlag, Node, Workload};

#[derive(PartialEq, Eq, Hash)]
pub enum OperationKind {
    MKDIR,
    CREATE,
    REMOVE,
}

impl OperationKind {
    pub fn all() -> HashSet<Self> {
        HashSet::from([
            OperationKind::CREATE,
            OperationKind::MKDIR,
            OperationKind::REMOVE,
        ])
    }
}

pub fn generate_new(rng: &mut impl Rng, size: usize) -> Workload {
    let mut executor = AbstractExecutor::new();
    for _ in 0..size {
        append_one(rng, &mut executor, OperationKind::all());
    }
    executor.recording
}

pub fn append_one(
    rng: &mut impl Rng,
    executor: &mut AbstractExecutor,
    pick_from: HashSet<OperationKind>,
) {
    let mode = vec![
        ModeFlag::S_IRWXU,
        ModeFlag::S_IRWXG,
        ModeFlag::S_IROTH,
        ModeFlag::S_IXOTH,
    ];
    let alive = executor.alive();
    let alive_dirs: Vec<DirIndex> = alive
        .iter()
        .filter_map(|n| match n {
            Node::DIR(dir) => Some(dir.clone()),
            Node::FILE(_) => None,
        })
        .collect();
    let alive_dirs_except_root: Vec<DirIndex> = alive_dirs
        .iter()
        .filter(|&&d| d != AbstractExecutor::root_index())
        .map(|d| d.clone())
        .collect();
    let mut banned_ops = HashSet::new();
    if alive_dirs_except_root.is_empty() {
        banned_ops.insert(OperationKind::REMOVE);
    }
    match pick_from.difference(&banned_ops).choose(rng).unwrap() {
        OperationKind::MKDIR => {
            executor
                .mkdir(
                    alive_dirs.choose(rng).unwrap(),
                    executor.nodes_created.to_string(),
                    mode.clone(),
                )
                .unwrap();
        }
        OperationKind::CREATE => {
            executor
                .create(
                    alive_dirs.choose(rng).unwrap(),
                    executor.nodes_created.to_string(),
                    mode.clone(),
                )
                .unwrap();
        }
        OperationKind::REMOVE => {
            let node = alive
                .iter()
                .filter(|n| match n {
                    Node::FILE(_) => true,
                    Node::DIR(dir) => *dir != AbstractExecutor::root_index(),
                })
                .choose(rng)
                .unwrap();
            executor.remove(node).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn test_generate_new() {
        for i in 0..1000 {
            let mut rng = StdRng::seed_from_u64(i);
            generate_new(&mut rng, 1000);
        }
    }
}
