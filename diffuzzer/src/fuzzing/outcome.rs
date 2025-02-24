/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::path::LocalPath;

pub struct Completed {
    pub stdout: String,
    pub stderr: String,
    pub dir: LocalPath,
}

pub enum Outcome {
    Completed(Completed),
    TimedOut { msg: String },
    Panicked,
    Skipped,
}
