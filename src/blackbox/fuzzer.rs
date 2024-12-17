use libafl::executors::ExitKind;
use log::info;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rand::SeedableRng;
use rand::prelude::StdRng;

use crate::abstract_fs::types::ConsolePipe;
use crate::blackbox::hasher_wrapper::Hasher;
use crate::config::Config;
use crate::harness::workload_harness;
use crate::mount::mount::FileSystemMount;
use crate::temp_dir::setup_temp_dir;
use crate::{abstract_fs::generator::generate_new, abstract_fs::types::Workload};

pub fn fuzz<FS: FileSystemMount>(
    count: usize,
    fst_fs: FS,
    snd_fs: FS,
    trace_len: usize,
    seed: u64,
    hasher: &Hasher,
    config: Config,
) {
    info!("running blackbox fuzzing");
    let mut rng = StdRng::seed_from_u64(seed);

    info!("setting up temporary directory");
    let temp_dir = setup_temp_dir();

    info!("setting up fuzzing components");
    let test_dir = temp_dir.clone();
    let exec_dir = temp_dir.join("exec");

    let fst_stdout = Rc::new(RefCell::new("".to_owned()));
    let fst_stderr = Rc::new(RefCell::new("".to_owned()));
    let snd_stdout = Rc::new(RefCell::new("".to_owned()));
    let snd_stderr = Rc::new(RefCell::new("".to_owned()));

    let fst_fs_dir = Path::new("/mnt")
        .join(fst_fs.to_string())
        .join("fstest")
        .into_boxed_path();
    let snd_fs_dir = Path::new("/mnt")
        .join(snd_fs.to_string())
        .join("fstest")
        .into_boxed_path();

    let fst_harness = get_workload_harness(
        fst_fs,
        test_dir.clone(),
        exec_dir.clone(),
        fst_fs_dir.clone(),
        fst_stdout,
        fst_stderr,
    );
    let snd_harness = get_workload_harness(
        snd_fs,
        test_dir.clone(),
        exec_dir.clone(),
        snd_fs_dir.clone(),
        snd_stdout,
        snd_stderr,
    );

    for _ in 1..=count {
        let workload = generate_new(&mut rng, trace_len, &config.operation_weights);
        fst_harness(&workload);
        snd_harness(&workload);
        hasher.compare_hash(&fst_fs_dir, &snd_fs_dir);
    }
}

fn get_workload_harness<FS: FileSystemMount>(
    fs: FS,
    test_dir: PathBuf,
    exec_dir: PathBuf,
    fs_dir: Box<Path>,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
) -> impl Fn(&Workload) -> ExitKind {
    return workload_harness(
        fs,
        test_dir.into_boxed_path(),
        exec_dir.into_boxed_path(),
        fs_dir,
        stdout,
        stderr,
    );
}
