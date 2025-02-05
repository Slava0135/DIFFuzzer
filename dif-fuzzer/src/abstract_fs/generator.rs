use rand::{seq::SliceRandom, Rng};

use super::{
    flags::ModeFlag,
    fs::AbstractFS,
    node::FileDescriptorIndex,
    operation::{OperationKind, OperationWeights},
    pathname::{Name, PathName},
    workload::Workload,
};

/// from AFL++ include/config.h
const INTERESTING_UNSIGNED: &[u64] = &[
    0, 1, // cool numbers
    16, 32, 64, 100, // one-off with common buffer size
    127, // overflow signed 8-bit when incremented
    128, // overflow signed 8-bit
    255, // overflow unsigned 8-bit when incremented
    256, // overflow unsigned 8-bit
    512, 1000, 1024, 4096,  // one-off with common buffer size
    32767, // overflow signed 16-bit when incremented
    32768, // overflow signed 16-bit
    65535, // overflow unsigned 16-bit when incremented
    65536, // overflow unsigned 16-bit
    100000,
];

fn random_interesting_unsigned(rng: &mut impl Rng) -> u64 {
    INTERESTING_UNSIGNED.choose(rng).unwrap().clone()
}

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
    let alive_open_files: Vec<FileDescriptorIndex> = alive
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
        ops.weights.retain(|(op, _)| *op != OperationKind::READ);
        ops.weights.retain(|(op, _)| *op != OperationKind::WRITE);
        ops.weights.retain(|(op, _)| *op != OperationKind::FSYNC);
    }
    match ops.weights.choose_weighted(rng, |item| item.1).unwrap().0 {
        OperationKind::MKDIR => {
            let path = alive.dirs.choose(rng).unwrap().to_owned();
            fs.mkdir(path.join(gen_name()), mode.clone()).unwrap();
        }
        OperationKind::CREATE => {
            let path = alive.dirs.choose(rng).unwrap().to_owned();
            fs.create(path.join(gen_name()), mode.clone()).unwrap();
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
            fs.hardlink(file_path, dir_path.join(gen_name())).unwrap();
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
            fs.rename(old_path, new_path.join(gen_name())).unwrap();
        }
        OperationKind::OPEN => {
            let path = alive_closed_files.choose(rng).unwrap().to_owned();
            fs.open(path).unwrap();
        }
        OperationKind::CLOSE => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.close(des).unwrap();
        }
        OperationKind::WRITE => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.read(des, random_interesting_unsigned(rng)).unwrap();
        }
        OperationKind::READ => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.write(
                des,
                random_interesting_unsigned(rng),
                random_interesting_unsigned(rng),
            )
            .unwrap();
        }
        OperationKind::FSYNC => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.fsync(des).unwrap();
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
