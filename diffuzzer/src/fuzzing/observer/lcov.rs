/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs;

use anyhow::Context;

use crate::{
    command::{CommandInterface, CommandWrapper},
    fuzzing::outcome::Completed,
    path::RemotePath,
};

use super::Observer;

const LCOV_INFO_FILE_NAME: &str = "lcov.info";

pub struct LCovObserver {
    source_dir: RemotePath,
}

impl Observer for LCovObserver {
    fn pre_exec(
        &mut self,
        cmdi: &dyn CommandInterface,
        output_dir: &RemotePath,
    ) -> anyhow::Result<()> {
        // Generate tracefile (from compiler-generated data) with all counter values set to zero.
        let mut lcov = CommandWrapper::new("lcov");
        lcov.arg("--zerocounters");
        lcov.arg("--directory").arg(self.source_dir.base.as_ref());
        cmdi.exec(lcov, None)
            .with_context(|| "failed to generate lcov tracefile")?;
        // Capture initial zero coverage data from the compile-time '.gcno' data files.
        let mut lcov = CommandWrapper::new("lcov");
        lcov.arg("--capture");
        lcov.arg("--initial");
        lcov.arg("--directory").arg(self.source_dir.base.as_ref());
        lcov.arg("--output-file")
            .arg(output_dir.join(LCOV_INFO_FILE_NAME).base.as_ref());
        cmdi.exec(lcov, None)
            .with_context(|| "failed to capture initial zero lcov coverage data")?;
        Ok(())
    }

    fn post_teardown(
        &mut self,
        cmdi: &dyn CommandInterface,
        output_dir: &RemotePath,
    ) -> anyhow::Result<()> {
        // Capture runtime coverage data.
        let mut lcov = CommandWrapper::new("lcov");
        lcov.arg("--capture");
        lcov.arg("--directory").arg(self.source_dir.base.as_ref());
        lcov.arg("--output-file")
            .arg(output_dir.join(LCOV_INFO_FILE_NAME).base.as_ref());
        cmdi.exec(lcov, None)
            .with_context(|| "failed to capture lcov coverage data")?;
        Ok(())
    }
}

impl LCovObserver {
    pub fn new(source_dir: RemotePath) -> Self {
        Self { source_dir }
    }
    pub fn read_lcov(outcome: &Completed) -> anyhow::Result<String> {
        fs::read_to_string(outcome.dir.join(LCOV_INFO_FILE_NAME))
            .with_context(|| "failed to read lcov file")
    }
}
