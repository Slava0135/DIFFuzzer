/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use rand::{Rng, seq::SliceRandom};

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
    *INTERESTING_UNSIGNED.choose(rng).unwrap()
}

/// Generates new random test workload of specified size.
pub fn generate_new(rng: &mut impl Rng, size: usize, weights: &OperationWeights) -> Workload {
    let mut fs = AbstractFS::new();
    let mut name_idx: usize = 0;
    let mut gen_name = || {
        let name = name_idx.to_string();
        name_idx += 1;
        name
    };
    for _ in 0..size {
        append_one(rng, &mut fs, weights, &mut gen_name);
    }
    fs.recording
}

/// Appends one random operation at the end of workload.
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
        .filter(|(idx, _)| *idx != AbstractFS::root_index())
        .map(|(_, path)| path)
        .cloned()
        .collect();
    let alive_closed_files: Vec<PathName> = alive
        .files
        .iter()
        .filter(|(idx, _)| fs.file(idx).descriptor.is_none())
        .map(|(_, path)| path)
        .cloned()
        .collect();
    let alive_open_files: Vec<FileDescriptorIndex> = alive
        .files
        .iter()
        .filter_map(|(idx, _)| fs.file(idx).descriptor)
        .collect();
    let mut ops = weights.clone();
    if alive_dirs_except_root.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::Remove);
    }
    if alive.files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::Hardlink);
    }
    if alive_dirs_except_root.is_empty() && alive.files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::Rename);
    }
    if alive_closed_files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::Open);
    }
    if alive_open_files.is_empty() {
        ops.weights.retain(|(op, _)| *op != OperationKind::Close);
        ops.weights.retain(|(op, _)| *op != OperationKind::Read);
        ops.weights.retain(|(op, _)| *op != OperationKind::Write);
        ops.weights.retain(|(op, _)| *op != OperationKind::FSync);
    }
    match ops.weights.choose_weighted(rng, |item| item.1).unwrap().0 {
        OperationKind::MkDir => {
            let path = alive.dirs.choose(rng).unwrap().to_owned().1;
            fs.mkdir(path.join(gen_name()), mode.clone()).unwrap();
        }
        OperationKind::Create => {
            let path = alive.dirs.choose(rng).unwrap().to_owned().1;
            fs.create(path.join(gen_name()), mode.clone()).unwrap();
        }
        OperationKind::Remove => {
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
        OperationKind::Hardlink => {
            let file_path = alive.files.choose(rng).unwrap().to_owned().1;
            let dir_path = alive.dirs.choose(rng).unwrap().to_owned().1;
            fs.hardlink(file_path, dir_path.join(gen_name())).unwrap();
        }
        OperationKind::Rename => {
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
                .filter(|(_, path)| !old_path.is_prefix_of(path))
                .map(|(_, path)| path)
                .cloned()
                .collect();
            let new_path = alive_non_subdirectories.choose(rng).unwrap().to_owned();
            fs.rename(old_path, new_path.join(gen_name())).unwrap();
            todo!("fix subdirectories rename with symlinks");
        }
        OperationKind::Open => {
            let path = alive_closed_files.choose(rng).unwrap().to_owned();
            fs.open(path).unwrap();
        }
        OperationKind::Close => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.close(des).unwrap();
        }
        OperationKind::Write => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.read(des, random_interesting_unsigned(rng)).unwrap();
        }
        OperationKind::Read => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.write(
                des,
                random_interesting_unsigned(rng),
                random_interesting_unsigned(rng),
            )
            .unwrap();
        }
        OperationKind::FSync => {
            let des = alive_open_files.choose(rng).unwrap().to_owned();
            fs.fsync(des).unwrap();
        }
        OperationKind::Symlink => {
            let target: PathName = [
                alive
                    .dirs
                    .iter()
                    .map(|(_, path)| path.clone())
                    .collect::<Vec<PathName>>(),
                alive
                    .files
                    .iter()
                    .map(|(_, path)| path.clone())
                    .collect::<Vec<PathName>>(),
            ]
            .concat()
            .choose(rng)
            .unwrap()
            .to_owned();
            let linkpath = alive.dirs.choose(rng).unwrap().1.clone();
            fs.symlink(target, linkpath.join(gen_name())).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use super::*;

    #[test]
    fn smoke_test_generate_new() {
        for i in 0..100 {
            let mut rng = StdRng::seed_from_u64(i);
            generate_new(&mut rng, 1000, &OperationWeights::uniform());
        }
    }
}
