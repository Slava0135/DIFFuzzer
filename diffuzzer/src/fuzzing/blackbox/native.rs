/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Ok};
use log::{debug, info};
use rand::prelude::StdRng;
use rand::SeedableRng;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::abstract_fs::generator::generate_new;
use crate::command::LocalCommandInterface;
use crate::config::Config;

use crate::fuzzing::fuzzer::Fuzzer;
use crate::fuzzing::runner::{parse_trace, Runner};
use crate::mount::mount::FileSystemMount;
use crate::path::LocalPath;

pub struct NativeBlackBoxFuzzer {
    runner: Runner,
    rng: StdRng,
}

impl NativeBlackBoxFuzzer {
    pub fn new(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
    ) -> Self {
        Self {
            runner: Runner::new(
                fst_mount,
                snd_mount,
                crashes_path,
                config,
                false,
                Box::new(LocalCommandInterface::new()),
            ),
            rng: StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
        }
    }
}

impl Fuzzer for NativeBlackBoxFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        debug!("generate input");
        let input = generate_new(
            &mut self.rng,
            self.runner.config.max_workload_length.into(),
            &self.runner.config.operation_weights,
        );

        let binary_path = self.runner().compile_test(&input)?;

        let (fst_outcome, snd_outcome) = self.runner().run_harness(&binary_path)?;

        let fst_trace =
            parse_trace(&fst_outcome).with_context(|| format!("failed to parse first trace"))?;
        let snd_trace =
            parse_trace(&snd_outcome).with_context(|| format!("failed to parse second trace"))?;

        if self.detect_errors(
            &input,
            &binary_path,
            &fst_trace,
            &snd_trace,
            &fst_outcome,
            &snd_outcome,
        )? {
            return Ok(());
        }

        self.do_objective(
            &input,
            &binary_path,
            &fst_trace,
            &snd_trace,
            &fst_outcome,
            &snd_outcome,
        )?;

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
