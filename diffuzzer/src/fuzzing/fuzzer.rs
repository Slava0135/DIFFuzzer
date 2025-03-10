/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::time::Instant;

use anyhow::Context;
use log::{debug, error, info, warn};

use crate::{
    abstract_fs::workload::Workload,
    fuzzing::runner::parse_trace,
    path::RemotePath,
};
use dash::FileDiff;

use super::{outcome::Completed, runner::Runner};

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
        fst_outcome: &Completed,
        snd_outcome: &Completed,
    ) -> anyhow::Result<bool> {
        let runner = self.runner();
        debug!("do objectives");
        let dash_is_interesting = runner
            .dash_objective
            .is_interesting()
            .with_context(|| "failed to do hash objective")?;

        let fst_trace =
            parse_trace(&fst_outcome.dir).with_context(|| "failed to parse first trace")?;
        let snd_trace =
            parse_trace(&snd_outcome.dir).with_context(|| "failed to parse second trace")?;

        let trace_is_interesting = runner
            .trace_objective
            .is_interesting(&fst_trace, &snd_trace)
            .with_context(|| "failed to do trace objective")?;
        if trace_is_interesting || dash_is_interesting {
            let reason = format!(
                "error detected by: trace?: {}, hash?: {}",
                trace_is_interesting, dash_is_interesting
            );
            debug!("{}", reason);
            let mut diff: Vec<FileDiff> = vec![];
            if dash_is_interesting {
                diff = runner.dash_objective.get_diff();
            }
            let dir_name = input.generate_name();
            runner
                .report_diff(
                    input,
                    dir_name,
                    binary_path,
                    runner.crashes_path.clone(),
                    diff,
                    fst_outcome,
                    snd_outcome,
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
        fst_outcome: &Completed,
        snd_outcome: &Completed,
    ) -> anyhow::Result<bool> {
        debug!("detect errors");

        let fst_trace =
            parse_trace(&fst_outcome.dir).with_context(|| "failed to parse first trace")?;
        let snd_trace =
            parse_trace(&snd_outcome.dir).with_context(|| "failed to parse second trace")?;

        if fst_trace.has_errors() && snd_trace.has_errors() {
            let reason = "both traces contain errors, potential bug in model".to_owned();
            warn!("{}", reason);
            let accidents_path = self.runner().accidents_path.clone();
            let dir_name = input.generate_name();
            self.runner()
                .report_diff(
                    input,
                    dir_name,
                    binary_path,
                    accidents_path,
                    vec![],
                    fst_outcome,
                    snd_outcome,
                    reason,
                )
                .with_context(|| "failed to report accident")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn report_crash(&mut self, input: &Workload, reason: String) -> anyhow::Result<()> {
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
