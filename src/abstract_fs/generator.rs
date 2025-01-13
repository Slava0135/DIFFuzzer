use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng,
};

use super::{
    executor::AbstractExecutor,
    flags::ModeFlag,
    node::{DirIndex, FileIndex, Name, Node},
    operation::{OperationKind, OperationWeights},
    workload::Workload,
};

pub fn generate_new(rng: &mut impl Rng, size: usize, weights: &OperationWeights) -> Workload {
    let mut executor = AbstractExecutor::new();
    let mut name_idx: usize = 0;
    let mut gen_name = || {
        let name = name_idx.to_string();
        name_idx += 1;
        name
    };
    for _ in 0..size {
        append_one(rng, &mut executor, &weights, &mut gen_name);
    }
    executor.recording
}

pub fn append_one(
    rng: &mut impl Rng,
    executor: &mut AbstractExecutor,
    weights: &OperationWeights,
    mut gen_name: impl FnMut() -> Name,
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
            _ => None,
        })
        .collect();
    let alive_files: Vec<FileIndex> = alive
        .iter()
        .filter_map(|n| match n {
            Node::FILE(file) => Some(file.clone()),
            _ => None,
        })
        .collect();
    let alive_dirs_except_root: Vec<DirIndex> = alive_dirs
        .iter()
        .filter(|&&d| d != AbstractExecutor::root_index())
        .map(|d| d.clone())
        .collect();
    let mut ops = weights.clone();
    if alive_dirs_except_root.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::REMOVE);
    }
    if alive_files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::HARDLINK);
    }
    match ops.weights.choose_weighted(rng, |item| item.1).unwrap().0 {
        OperationKind::MKDIR => {
            let dir = alive_dirs.choose(rng).unwrap();
            executor
                .mkdir(executor.make_path(dir, &gen_name()), mode.clone())
                .unwrap();
        }
        OperationKind::CREATE => {
            let dir = alive_dirs.choose(rng).unwrap();
            executor
                .create(executor.make_path(dir, &gen_name()), mode.clone())
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
            executor
                .remove(executor.resolve_path(node).pop().unwrap())
                .unwrap();
        }
        OperationKind::HARDLINK => {
            let file = alive_files.choose(rng).unwrap();
            let dir = alive_dirs.choose(rng).unwrap();
            executor
                .hardlink(
                    executor.resolve_file_path(file).pop().unwrap(),
                    executor.make_path(dir, &gen_name()),
                )
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn smoke_test_generate_new() {
        for i in 0..1000 {
            let mut rng = StdRng::seed_from_u64(i);
            generate_new(&mut rng, 1000, &OperationWeights::uniform());
        }
    }
}
