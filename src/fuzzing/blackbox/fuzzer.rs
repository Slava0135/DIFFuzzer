use anyhow::Context;
use log::{debug, info, warn};
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;
use std::{fs, io};

use rand::prelude::StdRng;
use rand::SeedableRng;

use crate::abstract_fs::generator::generate_new;
use crate::abstract_fs::trace::{Trace, TRACE_FILENAME};
use crate::config::Config;
use crate::fuzzing::abstract_fuzzer::fuzzer_data::FuzzData;
use crate::fuzzing::abstract_fuzzer::utils::{parse_trace, setup_dir};

use crate::harness::Harness;
use crate::hasher::hasher::{get_diff, get_hash_for_dir};
use crate::mount::mount::FileSystemMount;
use crate::save::{save_output, save_testcase};
use crate::temp_dir::setup_temp_dir;

pub struct BlackBoxFuzzer {
    data: FuzzData,
}

impl BlackBoxFuzzer {
    pub fn new(fst_fs: &'static dyn FileSystemMount, snd_fs: &'static dyn FileSystemMount) -> Self {
        Self {
            data: FuzzData::new(fst_fs, snd_fs),
        }
    }

    pub fn fuzz(&mut self, seed: u64, test_count: usize, config: Config) {
        let mut rng = StdRng::seed_from_u64(seed);
        let trace_len = config.max_workload_length as usize;

        for _ in 1..=test_count {
            let workload = generate_new(&mut rng, trace_len, &config.operation_weights);
            let wl_path = workload
                .compile(&self.data.test_dir)
                .with_context(|| "failed to compile test".to_string())
                .unwrap();

            debug!("running harness at '{}'", wl_path.display());

            setup_dir(self.data.fst_exec_dir.as_ref())
                .with_context(|| {
                    format!(
                        "failed to setup dir at '{}'",
                        self.data.fst_exec_dir.display()
                    )
                })
                .unwrap();
            setup_dir(self.data.snd_exec_dir.as_ref())
                .with_context(|| {
                    format!(
                        "failed to setup dir at '{}'",
                        self.data.snd_exec_dir.display()
                    )
                })
                .unwrap();

            self.data
                .fst_harness
                .run(&wl_path)
                .with_context(|| format!("failed to run first harness '{}'", self.data.fst_fs_name))
                .unwrap();
            self.data
                .snd_harness
                .run(&wl_path)
                .with_context(|| {
                    format!("failed to run second harness '{}'", self.data.snd_fs_name)
                })
                .unwrap();

            let fst_hash = get_hash_for_dir(self.data.fst_exec_dir.as_ref(), seed, false, false); //todo: options
            let snd_hash = get_hash_for_dir(self.data.snd_exec_dir.as_ref(), seed, false, false); //todo: options

            debug!("checking results");
            let fst_trace = parse_trace(&self.data.fst_trace_path)
                .with_context(|| format!("failed to parse first trace"))
                .unwrap();
            let snd_trace = parse_trace(&self.data.snd_trace_path)
                .with_context(|| format!("failed to parse second trace"))
                .unwrap();

            if fst_trace.has_errors() && snd_trace.has_errors() {
                warn!("both traces contain errors, potential bug in model");
                self.data
                    .report_crash(workload, &wl_path, self.data.accidents_path.clone())
                    .with_context(|| format!("failed to report accident"))
                    .unwrap();
                return;
            }

            let hash_diff_interesting = fst_hash != snd_hash;
            debug!("doing objectives");
            let console_is_interesting = self
                .data
                .console_objective
                .is_interesting()
                .with_context(|| format!("failed to do console objective"))
                .unwrap();
            let trace_is_interesting = self
                .data
                .trace_objective
                .is_interesting(&fst_trace, &snd_trace)
                .with_context(|| format!("failed to do trace objective"))
                .unwrap();

            if console_is_interesting || trace_is_interesting || hash_diff_interesting {
                self.data
                    .report_crash(workload, &wl_path, self.data.crashes_path.clone())
                    .with_context(|| format!("Can't report crush"))
                    .unwrap();
                self.data.show_stats();
                //todo: show(get_diff(&fst_exec_dir, &snd_exec_dir))
            }
        }
    }

}
