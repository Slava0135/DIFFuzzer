/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Ok};
use rand::SeedableRng;
use rand::prelude::StdRng;
use std::sync::mpsc::Sender;
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

use super::broker::BrokerMessage;

pub struct BlackBoxFuzzer {
    id: u8,
    runner: Runner,
    rng: StdRng,
    last_time_stats_sent: Instant,
    heartbeat_interval: u16,
    broker_tx: Sender<BrokerMessage>,
}

impl BlackBoxFuzzer {
    pub fn create(
        id: u8,
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        cmdi: Box<dyn CommandInterface>,
        supervisor: Box<dyn Supervisor>,
        local_tmp_dir: LocalPath,
        broker_tx: Sender<BrokerMessage>,
    ) -> anyhow::Result<Self> {
        let heartbeat_interval = config.heartbeat_interval;
        let runner = Runner::create(
            fst_mount,
            snd_mount,
            crashes_path,
            config,
            false,
            cmdi,
            supervisor,
            local_tmp_dir,
            (vec![], vec![]),
        )
        .with_context(|| "failed to create runner")?;
        Ok(Self {
            id,
            runner,
            rng: StdRng::seed_from_u64(
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64
            ),
            last_time_stats_sent: Instant::now(),
            heartbeat_interval,
            broker_tx,
        })
    }
}

impl Fuzzer for BlackBoxFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
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

    fn send_stats(&mut self, lazy: bool) -> anyhow::Result<()> {
        if !lazy || self.last_time_stats_sent.elapsed().as_secs() >= self.heartbeat_interval as u64
        {
            self.last_time_stats_sent = Instant::now();
            self.broker_tx
                .send(BrokerMessage::Stats {
                    id: self.id,
                    executions: self.runner.executions,
                    crashes: self.runner.crashes,
                })
                .with_context(|| "failed to send broker message")?
        }
        Ok(())
    }

    fn runner(&mut self) -> &mut Runner {
        &mut self.runner
    }
}
