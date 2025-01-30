use anyhow::{Context, Ok};
use log::{debug, info};
use rand::prelude::StdRng;
use rand::SeedableRng;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::abstract_fs::generator::generate_new;
use crate::config::Config;
use crate::fuzzing::common::{parse_trace, Fuzzer, Runner};

use crate::mount::mount::FileSystemMount;

pub struct BlackBoxFuzzer {
    runner: Runner,
    rng: StdRng,
}

impl BlackBoxFuzzer {
    pub fn new(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
    ) -> Self {
        Self {
            runner: Runner::new(fst_mount, snd_mount, config),
            rng: StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
        }
    }
}

impl Fuzzer for BlackBoxFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        debug!("generating input");
        let input = generate_new(
            &mut self.rng,
            self.runner.config.max_workload_length.into(),
            &self.runner.config.operation_weights,
        );

        let input_path = self.runner().compile_test(&input)?;

        self.runner().run_harness(&input_path)?;

        let fst_trace = parse_trace(&self.runner().fst_trace_path)
            .with_context(|| format!("failed to parse first trace"))?;
        let snd_trace = parse_trace(&self.runner().snd_trace_path)
            .with_context(|| format!("failed to parse second trace"))?;

        if self.detect_errors(&input, &input_path, &fst_trace, &snd_trace)? {
            return Ok(());
        }

        self.do_objective(&input, &input_path, &fst_trace, &snd_trace)?;

        self.runner().teardown_all()?;

        Ok(())
    }

    fn show_stats(&mut self) {
        self.runner.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.runner.stats.start);
        let secs = since_start.as_secs();
        info!(
            "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.runner.stats.crashes,
            self.runner.stats.executions,
            (self.runner.stats.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        );
    }

    fn runner(&mut self) -> &mut Runner {
        &mut self.runner
    }
}
