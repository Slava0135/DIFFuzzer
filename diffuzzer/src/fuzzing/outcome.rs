/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::abstract_fs::trace::Trace;
use crate::fuzzing::objective::dash::DashState;
use crate::fuzzing::runner::parse_trace;
use crate::path::LocalPath;
use anyhow::Context;

pub struct Completed {
    pub stdout: String,
    pub stderr: String,
    /// Directory with output files produced by test 
    pub dir: LocalPath,
    pub dash_state: DashState,
    pub trace: Trace,
}

impl Completed {
    pub fn new(stdout: String, stderr: String, dir: LocalPath, dash_state: DashState) -> Completed {
        let trace = parse_trace(&dir)
            .with_context(|| "failed to parse trace")
            .unwrap();
        Completed {
            stdout,
            stderr,
            dir,
            dash_state,
            trace,
        }
    }
}

pub enum Outcome {
    /// Test executed until the end.
    Completed(Completed),
    /// Test execution timed out.
    TimedOut,
    /// Test execution caused system shutdown / panic.
    Panicked,
    /// Test was not executed.
    Skipped,
}
