use anyhow::Context;
use log::{debug, info};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;
use std::{fs, io};

use rand::prelude::StdRng;
use rand::SeedableRng;

use crate::abstract_fs::generator::generate_new;
use crate::abstract_fs::trace::TRACE_FILENAME;
use crate::abstract_fs::types::Workload;
use crate::abstract_fuzzer::fuzzer_data::FuzzData;
use crate::config::Config;

use crate::harness::Harness;
use crate::hasher::hasher::{get_diff, get_hash_for_dir};
use crate::mount::mount::FileSystemMount;
use crate::save::{save_output, save_testcase};
use crate::temp_dir::setup_temp_dir;


pub struct BlackBoxFuzzer<FS1: FileSystemMount, FS2: FileSystemMount> {
    data: FuzzData<FS1, FS2>,
}

impl<FS1: FileSystemMount, FS2: FileSystemMount> BlackBoxFuzzer<FS1, FS2> {
    pub fn new(fst_fs: FS1, snd_fs: FS2) -> Self {
        Self {
            data: FuzzData::new(fst_fs, snd_fs)
        }
    }

    pub fn fuzz(
        &mut self,
        count: usize,
        trace_len: usize,
        seed: u64,
        config: Config,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);

        for _ in 1..=count {
            let workload = generate_new(&mut rng, trace_len, &config.operation_weights);
            let wl_path = workload
                .compile(&self.data.test_dir)
                .with_context(|| "failed to compile test".to_string())
                .unwrap();

            debug!("running harness at '{}'", wl_path.display());

            setup_dir(self.data.fst_exec_dir.as_ref())
                .with_context(|| format!("failed to setup dir at '{}'", self.data.fst_exec_dir.display()))
                .unwrap();
            setup_dir(self.data.snd_exec_dir.as_ref())
                .with_context(|| format!("failed to setup dir at '{}'", self.data.snd_exec_dir.display()))
                .unwrap();

            self.data.fst_harness
                .run(&wl_path)
                .with_context(|| format!("failed to run first harness '{}'", self.data.fst_fs_name))
                .unwrap();
            self.data.snd_harness
                .run(&wl_path)
                .with_context(|| format!("failed to run second harness '{}'", self.data.snd_fs_name))
                .unwrap();

            let fst_hash = get_hash_for_dir(self.data.fst_exec_dir.as_ref(), seed, false, false); //todo: options
            let snd_hash = get_hash_for_dir(self.data.snd_exec_dir.as_ref(), seed, false, false); //todo: options

            let hash_diff_interesting = fst_hash != snd_hash;
            debug!("doing objectives");
            let console_is_interesting = self.data.console_objective
                .is_interesting()
                .with_context(|| format!("failed to do console objective")).unwrap();
            let trace_is_interesting = self.data.trace_objective
                .is_interesting()
                .with_context(|| format!("failed to do trace objective")).unwrap();

            if console_is_interesting || trace_is_interesting || hash_diff_interesting {
                self.data.report_crash(workload, &wl_path);
                self.data.show_stats();
                //todo: get_diff(&fst_exec_dir, &snd_exec_dir, io::stdout(), false, false)
            }
        }
    }
}

//todo: move to utils
fn setup_dir(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path).unwrap_or(());
    fs::create_dir(path)
}
