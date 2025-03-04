/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use log::debug;
use dash::FileInfo;

use crate::abstract_fs::trace::{Trace, TraceRow};
use crate::fuzzing::objective::trace::TraceDiff::{DifferentLength, ExitCodeIsDifferent};

pub struct TraceObjective {}

impl TraceObjective {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceDiff {
    ExitCodeIsDifferent { fst: TraceRow, snd: TraceRow },
    DifferentLength,
}

impl TraceObjective {
    pub fn is_interesting(&mut self, fst_trace: &Trace, snd_trace: &Trace) -> anyhow::Result<bool> {
        debug!("do trace objective");
        Ok(!fst_trace.same_as(snd_trace))
    }

    pub fn get_diff(&mut self, fst_trace: &Trace, snd_trace: &Trace) -> Vec<TraceDiff> {
        let mut trace_diff: Vec<TraceDiff> = vec![];

        if fst_trace.rows.len() != snd_trace.rows.len() {
            trace_diff.push(DifferentLength);
            return trace_diff;
        }

        for i in 0..fst_trace.rows.len() {
            let mut fst_row = fst_trace.rows[i].clone();
            let mut snd_row = snd_trace.rows[i].clone();
            if fst_row != snd_row {
                fst_row.index = 0;
                snd_row.index = 0;
                trace_diff.push(ExitCodeIsDifferent {fst: fst_row, snd: snd_row})
            }
        }
        return trace_diff;
    }
}
