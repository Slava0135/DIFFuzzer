use log::info;
use std::cell::RefCell;
use std::{fs, io};
use std::path::Path;
use std::rc::Rc;
use anyhow::Context;

use rand::prelude::StdRng;
use rand::SeedableRng;

use crate::abstract_fs::generator::generate_new;
use crate::config::Config;
use crate::harness::Harness;
use crate::hasher::hasher::{get_diff, get_hash_for_dir};
use crate::mount::mount::FileSystemMount;
use crate::temp_dir::setup_temp_dir;

pub fn fuzz<FS1: FileSystemMount, FS2: FileSystemMount>(
    count: usize,
    fst_fs: FS1,
    snd_fs: FS2,
    trace_len: usize,
    seed: u64,
    config: Config,
) {
    info!("running blackbox fuzzing");
    let mut rng = StdRng::seed_from_u64(seed);

    let temp_dir = setup_temp_dir();

    info!("setting up fuzzing components");
    let test_dir = temp_dir.clone();
    let exec_dir = temp_dir.join("exec");

    let fst_stdout = Rc::new(RefCell::new("".to_owned()));
    let fst_stderr = Rc::new(RefCell::new("".to_owned()));
    let snd_stdout = Rc::new(RefCell::new("".to_owned()));
    let snd_stderr = Rc::new(RefCell::new("".to_owned()));

    let fst_exec_dir = temp_dir.join("fst_exec").into_boxed_path();
    let snd_exec_dir = temp_dir.join("snd_exec").into_boxed_path();

    let fst_fs_name = fst_fs.to_string();
    let snd_fs_name = snd_fs.to_string();

    let fst_harness = Harness::new(
        fst_fs,
        exec_dir.clone().into_boxed_path(),
        fst_exec_dir.clone(),
        fst_stdout,
        fst_stderr,
    );
    let snd_harness = Harness::new(
        snd_fs,
        exec_dir.clone().into_boxed_path(),
        snd_exec_dir.clone(),
        snd_stdout,
        snd_stderr,
    );



    for _ in 1..=count {
        let workload = generate_new(&mut rng, trace_len, &config.operation_weights);
        let wl_path = workload.compile(&test_dir)
            .with_context(|| "failed to compile test".to_string()).unwrap();

        setup_dir(fst_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", fst_exec_dir.display())).unwrap();
        setup_dir(snd_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", snd_exec_dir.display())).unwrap();

        fst_harness.run(&wl_path)
            .with_context(|| format!("failed to run first harness '{}'", fst_fs_name)).unwrap();
        snd_harness.run(&wl_path)
            .with_context(|| format!("failed to run second harness '{}'", snd_fs_name)).unwrap();

        let fst_hash = get_hash_for_dir(&fst_exec_dir, seed, false, false); //todo: options
        let snd_hash = get_hash_for_dir(&snd_exec_dir, seed, false, false); //todo: options

        //todo: cmp abstract state, traces and output
        if fst_hash != snd_hash { get_diff(&fst_exec_dir, &snd_exec_dir, io::stdout(), false, false) }
    }
}

//todo: move to utils
fn setup_dir(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path).unwrap_or(());
    fs::create_dir(path)
}

