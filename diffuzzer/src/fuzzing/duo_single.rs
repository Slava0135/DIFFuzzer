/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Ok};
use log::info;
use std::fs::read_to_string;
use std::time::Instant;

use crate::abstract_fs::workload::Workload;
use crate::config::Config;

use crate::fuzzing::fuzzer::Fuzzer;
use crate::fuzzing::outcome::DiffOutcome;
use crate::fuzzing::runner::Runner;
use crate::mount::FileSystemMount;
use crate::path::LocalPath;
use crate::reason::Reason;
use crate::supervisor::launch_cmdi_and_supervisor;

use super::blackbox::broker::BrokerHandle;

pub struct DuoSingleFuzzer {
    runner: Runner,
    test_path: LocalPath,
}

impl DuoSingleFuzzer {
    pub fn create(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        test_path: LocalPath,
        keep_fs: bool,
        no_qemu: bool,
    ) -> anyhow::Result<Self> {
        let local_tmp_dir = LocalPath::create_new_tmp("duo-single")?;

        let broker = BrokerHandle::Stub {
            start: Instant::now(),
        };
        let (cmdi, supervisor) =
            launch_cmdi_and_supervisor(no_qemu, &config, &local_tmp_dir, broker.clone())?;

        let runner = Runner::create(
            fst_mount,
            snd_mount,
            crashes_path,
            config,
            keep_fs,
            cmdi,
            supervisor,
            local_tmp_dir,
            broker,
            (vec![], vec![]),
        )
        .with_context(|| "failed to create runner")?;
        Ok(Self { runner, test_path })
    }
}

impl Fuzzer for DuoSingleFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        info!("read testcase at '{}'", self.test_path);
        let input = read_to_string(&self.test_path).with_context(|| "failed to read testcase")?;
        let input: Workload =
            serde_json::from_str(&input).with_context(|| "failed to parse json")?;

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

    fn send_stats(&mut self, _lazy: bool) -> anyhow::Result<()> {
        Ok(())
    }

    fn runner(&mut self) -> &mut Runner {
        &mut self.runner
    }
}
