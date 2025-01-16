use rand::{seq::SliceRandom, Rng};

use super::{
    fs::AbstractFS,
    flags::ModeFlag,
    node::FileDescriptor,
    operation::{OperationKind, OperationWeights},
    pathname::{Name, PathName},
    workload::Workload,
};

pub fn generate_new(rng: &mut impl Rng, size: usize, weights: &OperationWeights) -> Workload {
    let mut fs = AbstractFS::new();
    let mut name_idx: usize = 0;
    let mut gen_name = || {
        let name = name_idx.to_string();
        name_idx += 1;
        name
    };
    for _ in 0..size {
        append_one(rng, &mut fs, &weights, &mut gen_name);
    }
    fs.recording
}

pub fn append_one(
    rng: &mut impl Rng,
    fs: &mut AbstractFS,
    weights: &OperationWeights,
    mut gen_name: impl FnMut() -> Name,
) {
    let mode = vec![
        ModeFlag::S_IRWXU,
        ModeFlag::S_IRWXG,
        ModeFlag::S_IROTH,
        ModeFlag::S_IXOTH,
    ];
    let alive = fs.alive();
    let alive_dirs_except_root: Vec<PathName> = alive
        .dirs
        .iter()
        .filter(|d| **d != "/".into())
        .map(|d| d.clone())
        .collect();
    let alive_closed_files: Vec<PathName> = alive
        .files
        .iter()
        .filter(|(idx, _)| fs.file(idx).descriptor.is_none())
        .map(|(_, p)| p.clone())
        .collect();
    let alive_open_files: Vec<FileDescriptor> = alive
        .files
        .iter()
        .map(|(idx, _)| fs.file(idx).descriptor)
        .flatten()
        .collect();
    let mut ops = weights.clone();
    if alive_dirs_except_root.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::REMOVE);
    }
    if alive.files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::HARDLINK);
    }
    if alive_dirs_except_root.is_empty() && alive.files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::RENAME);
    }
    if alive_closed_files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::OPEN);
    }
    if alive_open_files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::CLOSE);
    }
    match ops.weights.choose_weighted(rng, |item| item.1).unwrap().0 {
        OperationKind::MKDIR => {
            let path = alive.dirs.choose(rng).unwrap().to_owned();
            fs.mkdir(path.join(gen_name()), mode.clone()).unwrap();
        }
        OperationKind::CREATE => {
            let path = alive.dirs.choose(rng).unwrap().to_owned();
            fs
                .create(path.join(gen_name()), mode.clone())
                .unwrap();
        }
        OperationKind::REMOVE => {
            let path = [
                alive_dirs_except_root,
                alive.files.iter().map(|(_, path)| path.clone()).collect(),
            ]
            .concat()
            .choose(rng)
            .unwrap()
            .to_owned();
            fs.remove(path).unwrap();
        }
        OperationKind::HARDLINK => {
            let file_path = alive.files.choose(rng).unwrap().to_owned().1;
            let dir_path = alive.dirs.choose(rng).unwrap().to_owned();
            fs
                .hardlink(file_path, dir_path.join(gen_name()))
                .unwrap();
        }
        OperationKind::RENAME => {
            let old_path = [
                alive_dirs_except_root,
                alive.files.iter().map(|(_, path)| path.clone()).collect(),
            ]
            .concat()
            .choose(rng)
            .unwrap()
            .to_owned();
            let alive_non_subdirectories: Vec<PathName> = alive
                .dirs
                .iter()
                .filter(|p| !old_path.is_prefix_of(p))
                .map(|p| p.clone())
                .collect();
            let new_path = alive_non_subdirectories.choose(rng).unwrap().to_owned();
            fs
                .rename(old_path, new_path.join(gen_name()))
                .unwrap();
        }
        OperationKind::OPEN => {
            let path = alive_closed_files.choose(rng).unwrap().to_owned();
            fs.open(path).unwrap();
        }
        OperationKind::CLOSE => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.close(des).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn smoke_test_generate_new() {
        for i in 0..100 {
            let mut rng = StdRng::seed_from_u64(i);
            generate_new(&mut rng, 1000, &OperationWeights::uniform());
        }
    }
}
