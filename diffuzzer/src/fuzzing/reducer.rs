/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs::read_to_string;

use anyhow::{Context, Ok};
use log::{info, warn};

use crate::{
    abstract_fs::{mutator::remove, trace::TraceDiff, workload::Workload},
    command::CommandInterface,
    config::Config,
    fuzzing::outcome::Outcome,
    mount::FileSystemMount,
    path::LocalPath,
    supervisor::Supervisor,
};

use super::runner::{DiffOutcome, Runner};

pub struct Reducer {
    runner: Runner,
}

impl Reducer {
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
        Ok(Self { runner })
    }

    pub fn run(&mut self, test_path: &LocalPath, output_dir: &LocalPath) -> anyhow::Result<()> {
        info!("read testcase at '{}'", test_path);
        let input = read_to_string(test_path).with_context(|| "failed to read testcase")?;
        let input: Workload =
            serde_json::from_str(&input).with_context(|| "failed to parse json")?;

        let binary_path = self.runner.compile_test(&input)?;

        match self.runner.run_harness(&binary_path)? {
            (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                let diff = self
                    .runner
                    .diff(fst_outcome, snd_outcome)
                    .with_context(|| "failed to produce diff outcome")?;
                if diff.any_interesting() {
                    self.reduce_by_diff(input, output_dir, diff)?;
                } else {
                    warn!("no diff found");
                }
            }
            _ => todo!("handle all outcomes"),
        };
        Ok(())
    }

    fn reduce_by_diff(
        &mut self,
        mut bugcase: Workload,
        output_dir: &LocalPath,
        diff: DiffOutcome,
    ) -> anyhow::Result<()> {
        info!("reduce by diff");
        let mut idx_to_remove = bugcase.ops.len() - 1;
        let mut last_binary_path = None;
        let mut last_diff = None;
        loop {
            if let Some(reduced) = remove(&bugcase, idx_to_remove) {
                let binary_path = self.runner.compile_test(&reduced)?;
                let variation_name = format!("variation-{}", idx_to_remove);
                match self.runner.run_harness(&binary_path)? {
                    (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                        let next_diff = self
                            .runner
                            .diff(fst_outcome, snd_outcome)
                            .with_context(|| "failed to produce diff outcome")?;
                        if !diff.any_interesting() {
                            continue;
                        }
                        if same_diff(&diff, &next_diff) {
                            bugcase = reduced;
                            last_binary_path = Some(binary_path);
                            last_diff = Some(next_diff);
                        } else {
                            let reason = format!(
                                "error detected by: trace?: {}, hash?: {}",
                                diff.trace_interesting(),
                                diff.dash_interesting()
                            );
                            self.runner
                                .report_diff(
                                    &bugcase,
                                    variation_name,
                                    &binary_path,
                                    output_dir.clone(),
                                    &next_diff,
                                    reason,
                                )
                                .with_context(|| "failed to report bug variation")?;
                        }
                    }
                    (Outcome::Panicked, _) => {
                        self.runner
                            .report_crash(
                                &reduced,
                                variation_name,
                                output_dir.clone(),
                                format!("Filesystem '{}' panicked", self.runner.fst_fs_name),
                            )
                            .with_context(|| "failed to report bug variation")?;
                    }
                    (_, Outcome::Panicked) => {
                        self.runner
                            .report_crash(
                                &reduced,
                                variation_name,
                                output_dir.clone(),
                                format!("Filesystem '{}' panicked", self.runner.snd_fs_name),
                            )
                            .with_context(|| "failed to report bug variation")?;
                    }
                    (Outcome::TimedOut, _) => {
                        self.runner
                            .report_crash(
                                &reduced,
                                variation_name,
                                output_dir.clone(),
                                format!(
                                    "Filesystem '{}' timed out after {}s",
                                    self.runner.fst_fs_name, self.runner.config.timeout
                                ),
                            )
                            .with_context(|| "failed to report bug variation")?;
                    }
                    (_, Outcome::TimedOut) => {
                        self.runner
                            .report_crash(
                                &reduced,
                                variation_name,
                                output_dir.clone(),
                                format!(
                                    "Filesystem '{}' timed out after {}s",
                                    self.runner.snd_fs_name, self.runner.config.timeout
                                ),
                            )
                            .with_context(|| "failed to report bug variation")?;
                    }
                    (Outcome::Skipped, _) => {}
                    (_, Outcome::Skipped) => {}
                };
            }
            if idx_to_remove == 0 {
                break;
            }
            idx_to_remove -= 1
        }
        let binary_path = last_binary_path.with_context(|| "failed to get bugcase binary path")?;
        let diff = last_diff.with_context(|| "failed to get bugcase diff")?;
        let reason = format!(
            "error detected by: trace?: {}, hash?: {}",
            diff.trace_interesting(),
            diff.dash_interesting()
        );
        self.runner
            .report_diff(
                &bugcase,
                "reduced".to_owned(),
                &binary_path,
                output_dir.clone(),
                &diff,
                reason,
            )
            .with_context(|| "failed to save reduced bugcase")?;
        Ok(())
    }
}

fn same_diff(old: &DiffOutcome, new: &DiffOutcome) -> bool {
    if old.trace_diff.len() != new.trace_diff.len() {
        return false;
    }
    for i in 0..old.trace_diff.len() {
        match (&old.trace_diff[i], &new.trace_diff[i]) {
            (TraceDiff::DifferentLength, TraceDiff::DifferentLength) => {}
            (
                TraceDiff::TraceRowIsDifferent {
                    fst: old_fst,
                    snd: old_snd,
                },
                TraceDiff::TraceRowIsDifferent {
                    fst: new_fst,
                    snd: new_snd,
                },
            ) => {
                if !(old_fst.ignore_index_equal(new_fst) && old_snd.ignore_index_equal(new_snd)) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    old.dash_diff == new.dash_diff
}
