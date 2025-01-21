use anyhow::Context;
use log::{debug, info, warn};
use rand::prelude::StdRng;
use rand::SeedableRng;
use std::time::Instant;

use crate::abstract_fs::generator::generate_new;
use crate::config::Config;
use crate::fuzzing::common::{parse_trace, setup_dir, FuzzData};

use crate::hasher::hasher::{calc_dir_hash, get_diff, FileDiff};
use crate::mount::mount::FileSystemMount;

pub struct BlackBoxFuzzer {
    data: FuzzData,
}

impl BlackBoxFuzzer {
    pub fn new(fst_fs: &'static dyn FileSystemMount, snd_fs: &'static dyn FileSystemMount) -> Self {
        Self {
            data: FuzzData::new(fst_fs, snd_fs),
        }
    }

    pub fn fuzz(&mut self, seed: u64, test_count: Option<u64>, config: Config) {
        let mut rng = StdRng::seed_from_u64(seed);
        let trace_len = config.max_workload_length as usize;

        match test_count {
            None => loop {
                self.fuzz_one(&mut rng, trace_len, &config);
            },
            Some(count) => {
                for _ in 0..count {
                    self.fuzz_one(&mut rng, trace_len, &config);
                }
            }
        }
    }

    fn fuzz_one(&mut self, rng: &mut StdRng, trace_len: usize, config: &Config) {
        let workload = generate_new(rng, trace_len, &config.operation_weights);
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
            .with_context(|| format!("failed to run second harness '{}'", self.data.snd_fs_name))
            .unwrap();

        let fst_hash =
            calc_dir_hash(self.data.fst_exec_dir.as_ref(), &self.data.hasher_options);
        let snd_hash =
            calc_dir_hash(self.data.snd_exec_dir.as_ref(), &self.data.hasher_options);

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
                .report_crash(workload, &wl_path, self.data.accidents_path.clone(), vec![])
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
            let mut diff: Vec<FileDiff> = vec![];
            if hash_diff_interesting {
                diff = get_diff(
                    &self.data.fst_exec_dir,
                    &self.data.snd_exec_dir,
                    &self.data.hasher_options,
                );
            }
            self.data
                .report_crash(workload, &wl_path, self.data.crashes_path.clone(), diff)
                .with_context(|| format!("failed to report crash"))
                .unwrap();
            self.show_stats();
        }
    }

    pub fn show_stats(&mut self) {
        self.data.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.data.stats.start);
        let secs = since_start.as_secs();
        info!(
            "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.data.stats.crashes,
            self.data.stats.executions,
            (self.data.stats.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        );
    }
}
