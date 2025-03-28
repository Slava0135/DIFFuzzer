/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::Context;

use crate::{
    abstract_fs::workload::Workload,
    command::{CommandInterface, CommandWrapper},
    path::RemotePath,
};

pub const TEST_SOURCE_FILENAME: &str = "test.c";
pub const TEST_EXE_FILENAME: &str = "test.out";

impl Workload {
    pub fn compile(
        &self,
        cmdi: &dyn CommandInterface,
        dir: &RemotePath,
    ) -> anyhow::Result<RemotePath> {
        let encoded = self.encode_c();
        let test_path = dir.join(TEST_SOURCE_FILENAME);
        let test_exec = dir.join(TEST_EXE_FILENAME);
        cmdi.write(&test_path, encoded.as_bytes())
            .with_context(|| format!("failed to write test source at '{}'", test_path))?;
        let mut make = CommandWrapper::new("make");
        make.arg("-C").arg(dir.base.as_ref());
        cmdi.exec(make, None)
            .with_context(|| "failed to compile test")?;
        Ok(test_exec)
    }
}
