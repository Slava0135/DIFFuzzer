/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use log::debug;

use crate::abstract_fs::trace::Trace;

pub struct TraceObjective {}

impl TraceObjective {
    pub fn new() -> Self {
        Self {}
    }
}

impl TraceObjective {
    pub fn is_interesting(&mut self, fst_trace: &Trace, snd_trace: &Trace) -> anyhow::Result<bool> {
        debug!("do trace objective");
        Ok(!fst_trace.same_as(snd_trace))
    }
}
