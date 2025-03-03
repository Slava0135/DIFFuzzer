/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{ffi::OsStr, fmt::Display, path::Path};

/// Prefix for temporary files to use
const TMP_DIR_PREFIX: &str = "diffuzzer";

/// Local paths always locate files on host (local) machine
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
    /// Create new temporary path with prefix added
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

/// Remote paths locate files on guest (remote) machine where tests are compiled and executed
/// or files on host (local) machine when QEMU is disabled.
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
    /// Create new temporary path with prefix added
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
