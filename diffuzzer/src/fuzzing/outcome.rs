/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::path::LocalPath;

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
    /// Test was not executed.
    Skipped,
}
