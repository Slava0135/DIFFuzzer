use rand::{
    seq::SliceRandom,
    Rng,
};

use super::{
    executor::{join_path, AbstractExecutor},
    flags::ModeFlag,
    node::{Name, PathName},
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
    let alive_dirs_except_root: Vec<PathName> = alive
        .dirs
        .iter()
        .filter(|d| **d != "/")
        .map(|d| d.clone())
        .collect();
    let mut ops = weights.clone();
    if alive_dirs_except_root.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::REMOVE);
    }
    if alive.files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::HARDLINK);
    }
    match ops.weights.choose_weighted(rng, |item| item.1).unwrap().0 {
        OperationKind::MKDIR => {
            let path = alive.dirs.choose(rng).unwrap().to_owned();
            executor
                .mkdir(join_path(path, gen_name()), mode.clone())
                .unwrap();
        }
        OperationKind::CREATE => {
            let path = alive.dirs.choose(rng).unwrap().to_owned();
            executor
                .create(join_path(path, gen_name()), mode.clone())
                .unwrap();
        }
        OperationKind::REMOVE => {
            let path = [alive_dirs_except_root, alive.files]
                .concat()
                .choose(rng)
                .unwrap()
                .to_owned();
            executor.remove(path).unwrap();
        }
        OperationKind::HARDLINK => {
            let file_path = alive.files.choose(rng).unwrap().to_owned();
            let dir_path = alive.dirs.choose(rng).unwrap().to_owned();
            executor
                .hardlink(file_path, join_path(dir_path, gen_name()))
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
