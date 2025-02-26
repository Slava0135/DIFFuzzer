/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::abstract_fs::trace::Trace;
use crate::fuzzing::objective::hash::HashHolder;
use crate::fuzzing::runner::parse_trace;
use crate::path::LocalPath;
use anyhow::Context;

pub struct Completed {
    pub stdout: String,
    pub stderr: String,
    pub dir: LocalPath,
    pub hash_holder: HashHolder,
    pub trace: Trace,
}

impl Completed {
    pub(crate) fn new(
        stdout: String,
        stderr: String,
        dir: LocalPath,
        hash_holder: HashHolder,
    ) -> Completed {
        let trace = parse_trace(&dir)
            .with_context(|| "failed to parse trace")
            .unwrap();
        Completed {
            stdout,
            stderr,
            dir,
            hash_holder,
            trace,
        }
    }
}

pub enum Outcome {
    Completed(Completed),
    TimedOut,
    Panicked,
    Skipped,
}
