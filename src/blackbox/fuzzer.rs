use std::cell::RefCell;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use libafl::executors::ExitKind;
use log::info;

use rand::prelude::StdRng;
use rand::SeedableRng;

use crate::{
    abstract_fs::generator::generate_new,
    abstract_fs::types::Workload,
    mount::btrfs::Btrfs,
    mount::ext4::Ext4,
    mount::mount::FileSystemMount,
    utils::harness::workload_harness,
    utils::temp_dir_actions::get_temp_dir,
};
use crate::abstract_fs::types::ConsolePipe;
use crate::blackbox::hasher_wrapper::Hasher;
use crate::config::Config;


pub fn fuzz_with_end<FS: FileSystemMount>(mut count: usize,
                                          fs_reference: FS,
                                          fs_target: FS,
                                          trace_len: usize,
                                          seed: u64,
                                          hasher: &Hasher,
                                          config: Config) {
    info!("running blackbox fuzzing");
    let mut rng = StdRng::seed_from_u64(seed);

    info!("setting up temporary directory");

    let temp_dir = get_temp_dir();
    info!("setting up fuzzing components");
    let test_dir = temp_dir.clone();
    let exec_dir = temp_dir.join("exec");

    let ref_stdout = Rc::new(RefCell::new("".to_owned()));
    let ref_stderr = Rc::new(RefCell::new("".to_owned()));
    let trg_stdout = Rc::new(RefCell::new("".to_owned()));
    let trg_stderr = Rc::new(RefCell::new("".to_owned()));

    let fs_trg_mnt = fs_target.mount_t();
    let fs_rfr_mnt = fs_reference.mount_t();

    let mut reference_harness = get_workload_harness(fs_reference, test_dir.clone(), exec_dir.clone(), ref_stdout, ref_stderr);
    let mut target_harness = get_workload_harness(fs_target, test_dir.clone(), exec_dir.clone(), trg_stdout, trg_stderr); //note: use tes_dir after or remove clone

    while count > 0 {
        let workload = generate_new(&mut rng, trace_len, &config.operation_weights);
        count -= 1;

        reference_harness(&workload);
        target_harness(&workload);
        //todo: dynamic get actual path?
        hasher.compare_hash(&*Path::new("/mnt").join(Path::new(&fs_trg_mnt)).join("fstest"),
                            &*Path::new("/mnt").join(Path::new(&fs_rfr_mnt)).join("fstest"));
    }
}

fn get_workload_harness<FS: FileSystemMount>(fs: FS,
                                             test_dir: PathBuf,
                                             exec_dir: PathBuf,
                                             stdout: ConsolePipe,
                                             stderr: ConsolePipe) -> impl Fn(&Workload) -> ExitKind {
    let fs_mnt = fs.mount_t();
    return workload_harness(
        fs,
        Path::new("/mnt")
            .join(fs_mnt)
            .join("fstest") //todo: changeable mountpoint?
            .into_boxed_path(),
        test_dir.into_boxed_path(),
        exec_dir.into_boxed_path(),
        stdout, stderr,
    );
}