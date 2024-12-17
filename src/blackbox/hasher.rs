use log::warn;
use std::path::Path;
use std::process::Command;
use std::str;

pub struct Hasher<'h> {
    pub path: &'h Path,
    pub options: &'h str,
}

impl Hasher<'_> {
    pub fn compare_hash(&self, target_path: &Path, ref_path: &Path) {
        let hash_target = self.calculate_hash(target_path);
        let hash_reference = self.calculate_hash(ref_path);
        if hash_target != hash_reference {
            warn!("Hash not equals");
            Command::new(self.path)
                .arg(self.options)
                .arg("-d")
                .arg(target_path)
                .arg(ref_path)
                .output()
                .expect("Error when difference calculating");
        }
    }

    pub fn calculate_hash(&self, path: &Path) -> Vec<u8> {
        let output = Command::new(self.path)
            .arg(self.options)
            .arg(path)
            .output()
            .expect("Error when hash calculating");
        if !output.status.success() {
            let err_str = match str::from_utf8(&output.stderr) {
                Ok(val) => val,
                Err(_) => panic!("got non UTF-8 data from stderr"),
            };
            warn!(
                "failed to eval abstract state for filesystem {}:{}",
                path.display(),
                err_str
            );
        }
        return output.stdout;
    }
}
