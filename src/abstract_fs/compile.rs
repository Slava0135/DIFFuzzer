use std::{fs, io, path::Path, process::Command};

use super::types::Workload;

pub const TEST_SOURCE_FILENAME: &str = "test.c";
pub const TEST_EXE_FILENAME: &str = "test.out";

impl Workload {
    pub fn compile(&self, dir: &Path) -> io::Result<Box<Path>> {
        let encoded = self.encode_c();
        let test_path = dir.join(TEST_SOURCE_FILENAME);
        let test_exec = dir.join(TEST_EXE_FILENAME);
        fs::write(test_path, encoded)?;
        let mut make = Command::new("make");
        make.arg("-C").arg(dir.as_os_str());
        make.output()?;
        Ok(test_exec.into_boxed_path())
    }
}
