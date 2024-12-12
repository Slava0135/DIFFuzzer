use std::env;
use std::path::{Path, PathBuf};
use libafl::executors::ExitKind;
use log::info;

use rand::prelude::StdRng;
use rand::SeedableRng;

use crate::{
    abstract_fs::generator::generate_new,
    abstract_fs::types::Workload,
    blackbox::executor::WorkloadExecutor,
    mount::btrfs::Btrfs,
    mount::ext4::Ext4,
    mount::mount::FileSystemMount,
    utils::harness::workload_harness,
};

pub fn fuzz_with_end<FS: FileSystemMount>(mut count: usize,
                                          fs_reference: FS,
                                          fs_target: FS,
                                          trace_len: usize,
                                          seed: u64) {
    info!("running blackbox fuzzing");
    info!("setting up temporary directory");
    let temp_dir = env::temp_dir().join("DIFFuzzer");
    std::fs::remove_dir_all(temp_dir.as_path()).unwrap_or(());
    std::fs::create_dir(temp_dir.as_path()).unwrap();

    let mut rng = StdRng::seed_from_u64(seed);

    info!("copying executor to '{}'", temp_dir.display());
    let executor_dir = Path::new("executor");
    let makefile = "makefile";
    let executor_h = "executor.h";
    let executor_cpp = "executor.cpp";
    std::fs::copy(executor_dir.join(makefile), temp_dir.join(makefile)).unwrap();
    std::fs::copy(executor_dir.join(executor_h), temp_dir.join(executor_h)).unwrap();
    std::fs::copy(executor_dir.join(executor_cpp), temp_dir.join(executor_cpp)).unwrap();

    info!("setting up fuzzing components");
    let test_dir = temp_dir.clone();
    let exec_dir = temp_dir.join("exec");

    let mut reference_harness = get_workload_harness(fs_reference, test_dir.clone(), exec_dir.clone());
    let mut target_harness = get_workload_harness(fs_target, test_dir.clone(), exec_dir.clone()); //note: use tes_dir after or remove clone

    while count > 0 {
        let workload = generate_new(&mut rng, trace_len);
        count -= 1;

        reference_harness(&workload);
        target_harness(&workload);
    }
}

fn get_workload_harness<FS: FileSystemMount>(fs: FS, test_dir: PathBuf, exec_dir: PathBuf) -> impl Fn(&Workload) -> ExitKind {
    return workload_harness(
        fs.clone(),
        Path::new("/mnt")
            .join(fs.mount_t())
            .join("fstest") //todo: changeable mountpoint?
            .into_boxed_path(),
        test_dir.into_boxed_path(),
        exec_dir.into_boxed_path(),
    );
}