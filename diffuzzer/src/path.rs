use std::{ffi::OsStr, fmt::Display, path::Path};

const TMP_DIR_PREFIX: &str = "diffuzzer";

#[derive(Clone)]
pub struct LocalPath {
    pub base: Box<Path>,
}

impl LocalPath {
    pub fn new(path: &Path) -> Self {
        Self {
            base: path.to_path_buf().into_boxed_path(),
        }
    }
    pub fn new_tmp(name: &str) -> Self {
        let base = Path::new("/tmp")
            .join(format!("{}-{}", TMP_DIR_PREFIX, name))
            .into_boxed_path();
        Self { base }
    }
    pub fn join<P: AsRef<Path>>(&self, path: P) -> Self {
        Self {
            base: self.base.join(path).into_boxed_path(),
        }
    }
    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> Self {
        Self {
            base: self.base.with_extension(extension).into_boxed_path(),
        }
    }
}

impl Display for LocalPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.base.display())
    }
}

impl AsRef<Path> for LocalPath {
    fn as_ref(&self) -> &Path {
        self.base.as_ref()
    }
}

#[derive(Clone)]
pub struct RemotePath {
    pub base: Box<Path>,
}

impl Display for RemotePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.base.display())
    }
}

impl RemotePath {
    pub fn new(path: &Path) -> Self {
        Self {
            base: path.to_path_buf().into_boxed_path(),
        }
    }
    pub fn new_tmp(name: &str) -> Self {
        let base = Path::new("/tmp")
            .join(format!("{}-{}", TMP_DIR_PREFIX, name))
            .into_boxed_path();
        Self { base }
    }
    pub fn join<P: AsRef<Path>>(&self, path: P) -> Self {
        Self {
            base: self.base.join(path).into_boxed_path(),
        }
    }
}
