/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::process::ExitStatus;

use crate::path::LocalPath;

pub struct Outcome {
    #[allow(dead_code)]
    pub exit_status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub dir: LocalPath,
}
