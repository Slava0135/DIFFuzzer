use std::{fs, io, path::Path, process::Command};

use super::{encode::encode_c, types::Workload};

impl Workload {
    pub fn compile(&self, dir: &Path) -> io::Result<Box<Path>> {
        let encoded = encode_c(self.clone());
        let test_path = dir.join("test.c");
        let test_exec = dir.join("test.out");
        fs::write(test_path, encoded)?;
        let mut make = Command::new("make");
        make.arg("-C").arg(dir.as_os_str());
        make.output()?;
        Ok(test_exec.into_boxed_path())
    }
}
