use std::{
    ffi::OsStr,
    fs,
    path::Path,
    process::{Command, Output},
};

use anyhow::{bail, Context};

pub trait CommandInterface {
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()>;
    fn exec(&self, cmd: CommandWrapper) -> anyhow::Result<Output>;
}

pub struct CommandWrapper {
    internal: Command,
}

impl CommandWrapper {
    pub fn new<S: AsRef<OsStr>>(cmd: S) -> Self {
        Self {
            internal: Command::new(cmd),
        }
    }
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.internal.arg(arg);
        self
    }
}

pub struct LocalCommandInterface {}

impl LocalCommandInterface {
    pub fn new() -> Self {
        LocalCommandInterface {}
    }
}

impl CommandInterface for LocalCommandInterface {
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("failed to create local dir at '{}'", path.display()))
    }
    fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove local dir at '{}'", path.display()))
    }
    fn exec(&self, mut cmd: CommandWrapper) -> anyhow::Result<Output> {
        let output = cmd
            .internal
            .output()
            .with_context(|| format!("failed to run command: {:?}", cmd.internal))?;
        if !output.status.success() {
            bail!(
                "command {:?} execution ended with error:\n{}",
                cmd.internal,
                String::from_utf8(output.stderr).unwrap_or("<invalid UTF-8 string>".into())
            );
        }
        Ok(output)
    }
}

pub struct RemoteCommandInterface {}

impl RemoteCommandInterface {
    pub fn new() -> Self {
        RemoteCommandInterface {}
    }
}

impl CommandInterface for RemoteCommandInterface {
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        todo!()
    }
    fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        todo!()
    }
    fn exec(&self, cmd: CommandWrapper) -> anyhow::Result<Output> {
        todo!()
    }
}
