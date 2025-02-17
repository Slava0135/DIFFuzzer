/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::time::Instant;

use anyhow::Context;
use log::{debug, error, info, warn};

use crate::{
    abstract_fs::{trace::Trace, workload::Workload},
    path::RemotePath,
};
use hasher::FileDiff;

use super::{outcome::Outcome, runner::Runner};

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
        fst_trace: &Trace,
        snd_trace: &Trace,
        fst_outcome: &Outcome,
        snd_outcome: &Outcome,
    ) -> anyhow::Result<bool> {
        let runner = self.runner();
        debug!("do objectives");
        let hash_diff_interesting = runner
            .hash_objective
            .is_interesting()
            .with_context(|| "failed to do hash objective")?;
        let trace_is_interesting = runner
            .trace_objective
            .is_interesting(fst_trace, snd_trace)
            .with_context(|| "failed to do trace objective")?;
        if trace_is_interesting || hash_diff_interesting {
            debug!(
                "error detected by: trace?: {}, hash?: {}",
                trace_is_interesting, hash_diff_interesting
            );
            let mut diff: Vec<FileDiff> = vec![];
            if hash_diff_interesting {
                diff = runner.hash_objective.get_diff();
            }
            runner
                .report_crash(
                    input,
                    binary_path,
                    runner.crashes_path.clone(),
                    diff,
                    fst_outcome,
                    snd_outcome,
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
        fst_trace: &Trace,
        snd_trace: &Trace,
        fst_outcome: &Outcome,
        snd_outcome: &Outcome,
    ) -> anyhow::Result<bool> {
        debug!("detect errors");
        if fst_trace.has_errors() && snd_trace.has_errors() {
            warn!("both traces contain errors, potential bug in model");
            let accidents_path = self.runner().accidents_path.clone();
            self.runner()
                .report_crash(
                    input,
                    binary_path,
                    accidents_path,
                    vec![],
                    fst_outcome,
                    snd_outcome,
                )
                .with_context(|| "failed to report accident")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn show_stats(&mut self);

    fn runner(&mut self) -> &mut Runner;
}
