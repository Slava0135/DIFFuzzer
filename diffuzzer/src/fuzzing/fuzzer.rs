/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::time::Instant;

use anyhow::Context;
use log::{error, info, warn};

use crate::{abstract_fs::workload::Workload, path::RemotePath, reason::Reason};

use super::{outcome::DiffCompleted, runner::Runner};

pub trait Fuzzer {
    fn run(&mut self, test_count: Option<u64>) {
        info!("start fuzzing loop");
        self.runner().stats.start = Instant::now();
        match test_count {
            None => loop {
                if self.runs() {
                    return;
                }
            },
            Some(count) => {
                for _ in 0..count {
                    if self.runs() {
                        return;
                    }
                }
            }
        }
    }

    fn runs(&mut self) -> bool {
        match self.fuzz_one() {
            Err(err) => {
                error!("{:?}", err);
                return true;
            }
            _ => self.runner().stats.executions += 1,
        }
        if Instant::now()
            .duration_since(self.runner().stats.last_time_showed)
            .as_secs()
            > self.runner().config.heartbeat_interval.into()
        {
            self.show_stats();
        }
        false
    }

    fn fuzz_one(&mut self) -> anyhow::Result<()>;

    fn do_objective(
        &mut self,
        input: &Workload,
        binary_path: &RemotePath,
        diff: &DiffCompleted,
    ) -> anyhow::Result<bool> {
        let runner = self.runner();
        if diff.any_interesting() {
            let mut reason = Reason::new();
            if diff.trace_interesting() {
                reason.md.heading("Trace Difference Found".to_owned());
                reason.add_trace_diff(&diff.trace_diff);
            }
            if diff.dash_interesting() {
                reason.md.heading("Dash Difference Found".to_owned());
                reason.add_dash_diff(&diff.dash_diff);
            }
            let dir_name = input.generate_name();
            runner
                .report_diff(
                    input,
                    dir_name,
                    binary_path,
                    runner.crashes_path.clone(),
                    diff,
                    reason,
                )
                .with_context(|| "failed to report crash")?;
            self.runner().stats.crashes += 1;
            self.show_stats();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn detect_errors(
        &mut self,
        input: &Workload,
        binary_path: &RemotePath,
        diff: &DiffCompleted,
    ) -> anyhow::Result<bool> {
        let fst_errors = diff.fst_trace.errors();
        let snd_errors = diff.snd_trace.errors();

        if !fst_errors.is_empty() && !snd_errors.is_empty() {
            let reason_str = "Both traces contain errors, potential bug in model".to_owned();
            let mut reason = Reason::new();
            warn!("{}", reason_str.to_lowercase());
            reason.md.heading(reason_str);
            reason.add_trace_rows(&fst_errors);
            reason.add_trace_rows(&snd_errors);
            let accidents_path = self.runner().accidents_path.clone();
            let dir_name = input.generate_name();
            self.runner()
                .report_diff(input, dir_name, binary_path, accidents_path, diff, reason)
                .with_context(|| "failed to report accident")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn report_crash(&mut self, input: &Workload, reason: Reason) -> anyhow::Result<()> {
        let dir_name = input.generate_name();
        let crashes_dir = self.runner().crashes_path.clone();
        self.runner()
            .report_crash(input, dir_name, crashes_dir, reason)
            .with_context(|| "failed to report panic")?;
        self.runner().stats.crashes += 1;
        self.show_stats();
        Ok(())
    }

    fn show_stats(&mut self);

    fn runner(&mut self) -> &mut Runner;
}
