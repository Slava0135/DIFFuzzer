use std::{fs, path::Path, process::Command};

use anyhow::{bail, Context};

use super::workload::Workload;

pub const TEST_SOURCE_FILENAME: &str = "test.c";
pub const TEST_EXE_FILENAME: &str = "test.out";

impl Workload {
    pub fn compile(&self, dir: &Path) -> anyhow::Result<Box<Path>> {
        let encoded = self.encode_c();
        let test_path = dir.join(TEST_SOURCE_FILENAME);
        let test_exec = dir.join(TEST_EXE_FILENAME);
        fs::write(&test_path, encoded)
            .with_context(|| format!("failed to write test source at '{}'", test_path.display()))?;
        let mut make = Command::new("make");
        make.arg("-C").arg(dir.as_os_str());
        let output = make.output().with_context(|| {
            format!(
                "failed to run makefile command at '{}': '{:?}'",
                dir.display(),
                make
            )
        })?;
        if !output.status.success() {
            bail!(
                "compilation failed with code {}",
                output.status.code().unwrap_or(-1)
            );
        }
        Ok(test_exec.into_boxed_path())
    }
}
