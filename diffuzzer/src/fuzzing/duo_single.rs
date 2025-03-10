/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Ok};
use log::info;
use std::fs::read_to_string;

use crate::abstract_fs::workload::Workload;
use crate::command::CommandInterface;
use crate::config::Config;

use crate::fuzzing::fuzzer::Fuzzer;
use crate::fuzzing::outcome::Outcome;
use crate::fuzzing::runner::Runner;
use crate::mount::FileSystemMount;
use crate::path::LocalPath;
use crate::supervisor::Supervisor;

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
        cmdi: Box<dyn CommandInterface>,
        supervisor: Box<dyn Supervisor>,
    ) -> anyhow::Result<Self> {
        let runner = Runner::create(
            fst_mount,
            snd_mount,
            crashes_path,
            config,
            keep_fs,
            cmdi,
            supervisor,
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
            (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                if self.detect_errors(&input, &binary_path, &fst_outcome, &snd_outcome)? {
                    return Ok(());
                }

                self.do_objective(&input, &binary_path, &fst_outcome, &snd_outcome)?;
            }
            (Outcome::Panicked, _) => {
                self.report_crash(
                    &input,
                    format!("Filesystem '{}' panicked", self.runner.fst_fs_name),
                )?;
            }
            (_, Outcome::Panicked) => {
                self.report_crash(
                    &input,
                    format!("Filesystem '{}' panicked", self.runner.snd_fs_name),
                )?;
            }
            (Outcome::TimedOut, _) => {
                self.report_crash(
                    &input,
                    format!(
                        "Filesystem '{}' timed out after {}s",
                        self.runner.fst_fs_name, self.runner.config.timeout
                    ),
                )?;
            }
            (_, Outcome::TimedOut) => {
                self.report_crash(
                    &input,
                    format!(
                        "Filesystem '{}' timed out after {}s",
                        self.runner.snd_fs_name, self.runner.config.timeout
                    ),
                )?;
            }
            (Outcome::Skipped, _) => {}
            (_, Outcome::Skipped) => {}
        };

        Ok(())
    }

    fn show_stats(&mut self) {}

    fn runner(&mut self) -> &mut Runner {
        &mut self.runner
    }
}
