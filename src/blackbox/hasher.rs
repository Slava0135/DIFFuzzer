use log::warn;
use std::path::Path;
use std::process::Command;
use std::str;

pub struct Hasher {
    pub path: Box<Path>,
    pub options: String,
}

impl Hasher {
    pub fn compare(&self, fst_path: &Path, snd_path: &Path) {
        let hash_target = self.eval(fst_path);
        let hash_reference = self.eval(snd_path);
        if hash_target != hash_reference {
            Command::new(self.path.as_ref())
                .arg(&self.options)
                .arg("-d")
                .arg(fst_path)
                .arg(snd_path)
                .output()
                .expect("error when comparing hashes");
        }
    }

    fn eval(&self, path: &Path) -> Vec<u8> {
        let output = Command::new(self.path.as_ref())
            .arg(&self.options)
            .arg(path)
            .output()
            .expect("error when evaluating hash");
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
