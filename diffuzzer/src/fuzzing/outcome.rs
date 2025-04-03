/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use dash::FileDiff;

use crate::{
    abstract_fs::trace::{Trace, TraceDiff},
    path::LocalPath,
};

pub struct Completed {
    pub stdout: String,
    pub stderr: String,
    /// Directory with output files produced by test
    pub dir: LocalPath,
}

impl Completed {
    pub fn new(stdout: String, stderr: String, dir: LocalPath) -> Completed {
        Completed {
            stdout,
            stderr,
            dir,
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
}

pub struct DiffCompleted {
    pub dash_diff: Vec<FileDiff>,
    pub trace_diff: Vec<TraceDiff>,
    pub fst_outcome: Completed,
    pub snd_outcome: Completed,
    pub fst_trace: Trace,
    pub snd_trace: Trace,
}

impl DiffCompleted {
    pub fn any_interesting(&self) -> bool {
        self.dash_interesting() || self.trace_interesting()
    }

    pub fn dash_interesting(&self) -> bool {
        !self.dash_diff.is_empty()
    }

    pub fn trace_interesting(&self) -> bool {
        !self.trace_diff.is_empty()
    }

    pub fn get_last_diff_trace_row(&self) -> Option<u32> {
        let mut res: Option<u32> = None;
        for bug in &self.trace_diff {
            match bug {
                TraceDiff::TraceRowIsDifferent { fst: f, snd: _ } => {
                    if res.is_none_or(|max| f.index > max) {
                        res = Some(f.index)
                    }
                }
                TraceDiff::DifferentLength => {
                    return None;
                }
            }
        }
        res
    }
}

pub enum DiffOutcome {
    DiffCompleted(DiffCompleted),
    FirstTimedOut { fs_name: String, timeout: u8 },
    SecondTimedOut { fs_name: String, timeout: u8 },
    FirstPanicked { fs_name: String },
    SecondPanicked { fs_name: String },
}
