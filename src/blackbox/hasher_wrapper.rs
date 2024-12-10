use std::path::Path;
use std::process::Command;
use log::warn;
use crate::blackbox::executor::ExecResults;

const HASHER: &Path = Path::new("./asfs");
const HASHER_EXIST: bool = HASHER.exists();
const HASHER_OPTS: &str = "-ml"; //todo: from env or config

pub fn compare_hash(outputs: &ExecResults) {
    if HASHER_EXIST {
        let hash_target = calculate_hash(outputs.workload_executor.target_path.as_ref());
        let hash_reference = calculate_hash(outputs.workload_executor.ref_path.as_ref());
        if hash_target != hash_reference {
            warn!("Hash not equals");
            Command::new(HASHER)
                .arg(HASHER_OPTS)
                .arg("-d")
                .arg(outputs.workload_executor.target_path.as_ref())
                .arg(outputs.workload_executor.ref_path.as_ref())
                .output()?;
        }
    }
}

pub fn calculate_hash(path: &Path) -> Vec<u8> {
    let exec = Command::new(HASHER).arg(HASHER_OPTS).arg(path);
    let output = exec.output()?;
    if !output.status.success() {
        let err_str = match str::from_utf8(&output.stderr) {
            Ok(val) => val,
            Err(_) => panic!("got non UTF-8 data from stderr"),
        };
        warn!("failed to eval abstract state for filesystem {}:{}", path, err_str);
    }
    let hash = match str::from_utf8(&output.stdout) {
        Ok(val) => val,
        Err(_) => panic!("got non UTF-8 data from stdout"),
    };
    return hash;
}