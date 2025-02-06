use std::{fs, process::Command};

use anyhow::{bail, Context};

use crate::{abstract_fs::workload::Workload, path::RemotePath};

pub const TEST_SOURCE_FILENAME: &str = "test.c";
pub const TEST_EXE_FILENAME: &str = "test.out";

impl Workload {
    pub fn compile(&self, dir: &RemotePath) -> anyhow::Result<RemotePath> {
        todo!("use cmdi");
        let encoded = self.encode_c();
        let test_path = dir.join(TEST_SOURCE_FILENAME);
        let test_exec = dir.join(TEST_EXE_FILENAME);
        fs::write(&test_path.base, encoded)
            .with_context(|| format!("failed to write test source at '{}'", test_path))?;
        let mut make = Command::new("make");
        make.arg("-C").arg(dir.base.as_ref());
        let output = make.output().with_context(|| {
            format!("failed to run makefile command at '{}': '{:?}'", dir, make)
        })?;
        if !output.status.success() {
            bail!(
                "compilation failed with code {}",
                output.status.code().unwrap_or(-1)
            );
        }
        Ok(test_exec)
    }
}
