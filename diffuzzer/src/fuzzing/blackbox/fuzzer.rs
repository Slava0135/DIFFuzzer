/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Ok};
use log::{debug, info};
use rand::SeedableRng;
use rand::prelude::StdRng;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::abstract_fs::generator::generate_new;
use crate::command::CommandInterface;
use crate::config::Config;

use crate::fuzzing::fuzzer::Fuzzer;
use crate::fuzzing::outcome::DiffOutcome;
use crate::fuzzing::runner::Runner;
use crate::mount::FileSystemMount;
use crate::path::LocalPath;
use crate::reason::Reason;
use crate::supervisor::Supervisor;

pub struct BlackBoxFuzzer {
    runner: Runner,
    rng: StdRng,
}

impl BlackBoxFuzzer {
    pub fn create(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        cmdi: Box<dyn CommandInterface>,
        supervisor: Box<dyn Supervisor>,
    ) -> anyhow::Result<Self> {
        let runner = Runner::create(
            fst_mount,
            snd_mount,
            crashes_path,
            config,
            false,
            cmdi,
            supervisor,
            (vec![], vec![]),
        )
        .with_context(|| "failed to create runner")?;
        Ok(Self {
            runner,
            rng: StdRng::seed_from_u64(
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64
            ),
        })
    }
}

impl Fuzzer for BlackBoxFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        debug!("generate input");
        let input = generate_new(
            &mut self.rng,
            self.runner.config.max_workload_length.into(),
            &self.runner.config.operation_weights,
        );

        let binary_path = self.runner().compile_test(&input)?;

        match self.runner().run_harness(&binary_path)? {
            DiffOutcome::DiffCompleted(diff) => {
                if self.detect_errors(&input, &binary_path, &diff)? {
                    return Ok(());
                }
                self.do_objective(&input, &binary_path, &diff)?;
            }
            DiffOutcome::FirstPanicked { fs_name } => {
                let mut reason = Reason::new();
                reason
                    .md
                    .heading(format!("Filesystem '{}' panicked", fs_name));
                self.report_crash(&input, reason)?;
            }
            DiffOutcome::SecondPanicked { fs_name } => {
                let mut reason = Reason::new();
                reason
                    .md
                    .heading(format!("Filesystem '{}' panicked", fs_name));
                self.report_crash(&input, reason)?;
            }
            DiffOutcome::FirstTimedOut { fs_name, timeout } => {
                let mut reason = Reason::new();
                reason.md.heading(format!(
                    "Filesystem '{}' timed out after {}s",
                    fs_name, timeout
                ));
                self.report_crash(&input, reason)?;
            }
            DiffOutcome::SecondTimedOut { fs_name, timeout } => {
                let mut reason = Reason::new();
                reason.md.heading(format!(
                    "Filesystem '{}' timed out after {}s",
                    fs_name, timeout
                ));
                self.report_crash(&input, reason)?;
            }
        };

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
