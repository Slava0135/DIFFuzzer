use std::path::Path;
use std::process::Command;

pub struct Hasher {
    pub path: Box<Path>,
    pub options: String,
}

enum HasherError {
    Eval(String),
}

impl Hasher {
    pub fn compare(&self, fst_path: &Path, snd_path: &Path) {
        let fst_hash = self.eval(fst_path);
        let snd_hash = self.eval(snd_path);
        match (fst_hash, snd_hash) {
            (Ok(fst_hash), Ok(snd_hash)) if fst_hash != snd_hash => {
                Command::new(self.path.as_ref())
                    .arg(&self.options)
                    .arg("-d")
                    .arg(fst_path)
                    .arg(snd_path)
                    .output()
                    .expect("error when comparing hashes");
            }
            _ => {}
        }
    }

    fn eval(&self, path: &Path) -> Result<String, HasherError> {
        let output = Command::new(self.path.as_ref())
            .arg(&self.options)
            .arg(path)
            .output()
            .expect("error when evaluating hash");
        if !output.status.success() {
            return Err(HasherError::Eval(
                String::from_utf8(output.stderr).unwrap_or("error reading stderr".to_owned()),
            ));
        }
        match String::from_utf8(output.stdout) {
            Ok(hash) => Ok(hash),
            Err(err) => Err(HasherError::Eval(err.to_string())),
        }
    }
}
