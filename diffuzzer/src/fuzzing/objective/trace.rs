/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::abstract_fs::trace::TraceDiff::{DifferentLength, TraceRowIsDifferent};
use crate::abstract_fs::trace::{Trace, TraceDiff};

pub struct TraceObjective {}

impl TraceObjective {
    pub fn new() -> Self {
        Self {}
    }
}

impl TraceObjective {
    pub fn get_diff(&mut self, fst_trace: &Trace, snd_trace: &Trace) -> Vec<TraceDiff> {
        let mut trace_diff: Vec<TraceDiff> = vec![];

        if fst_trace.rows.len() != snd_trace.rows.len() {
            trace_diff.push(DifferentLength);
            return trace_diff;
        }

        for i in 0..fst_trace.rows.len() {
            let fst_row = fst_trace.rows[i].clone();
            let snd_row = snd_trace.rows[i].clone();
            if !fst_row.ignore_index_equal(&snd_row) {
                trace_diff.push(TraceRowIsDifferent {
                    fst: fst_row,
                    snd: snd_row,
                })
            }
        }
        trace_diff
    }
}
