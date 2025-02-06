use std::{
    ffi::OsStr,
    fs,
    process::{Command, Output},
};

use anyhow::{bail, Context, Ok};

use crate::{
    config::QemuConfig,
    path::{LocalPath, RemotePath},
};

pub trait CommandInterface {
    fn create_dir_all(&self, path: &RemotePath) -> anyhow::Result<()>;
    fn remove_dir_all(&self, path: &RemotePath) -> anyhow::Result<()>;
    fn copy_to_remote(
        &self,
        local_path: &LocalPath,
        remote_path: &RemotePath,
    ) -> anyhow::Result<()>;
    fn copy_from_remote(
        &self,
        remote_path: &RemotePath,
        local_path: &LocalPath,
    ) -> anyhow::Result<()>;

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
    fn exec_local(mut self) -> anyhow::Result<Output> {
        let output = self
            .internal
            .output()
            .with_context(|| format!("failed to run local command: {:?}", self.internal))?;
        if !output.status.success() {
            bail!(
                "local command {:?} execution ended with error:\n{}",
                self.internal,
                String::from_utf8(output.stderr).unwrap_or("<invalid UTF-8 string>".into())
            );
        }
        Ok(output)
    }
}

pub struct LocalCommandInterface {}

impl LocalCommandInterface {
    pub fn new() -> Self {
        LocalCommandInterface {}
    }
}

impl CommandInterface for LocalCommandInterface {
    fn create_dir_all(&self, path: &RemotePath) -> anyhow::Result<()> {
        fs::create_dir_all(path.base.as_ref())
            .with_context(|| format!("failed to create local dir at '{}'", path))
    }
    fn remove_dir_all(&self, path: &RemotePath) -> anyhow::Result<()> {
        fs::remove_dir_all(path.base.as_ref())
            .with_context(|| format!("failed to remove local dir at '{}'", path))
    }
    fn copy_to_remote(
        &self,
        local_path: &LocalPath,
        remote_path: &RemotePath,
    ) -> anyhow::Result<()> {
        fs::copy(local_path, remote_path.base.as_ref()).with_context(|| {
            format!(
                "failed to copy local file from '{}' to '{}'",
                local_path,
                remote_path,
            )
        })?;
        Ok(())
    }
    fn copy_from_remote(
        &self,
        remote_path: &RemotePath,
        local_path: &LocalPath,
    ) -> anyhow::Result<()> {
        fs::copy(remote_path.base.as_ref(), local_path).with_context(|| {
            format!(
                "failed to copy local file from '{}' to '{}'",
                remote_path,
                local_path,
            )
        })?;
        Ok(())
    }
    fn exec(&self, cmd: CommandWrapper) -> anyhow::Result<Output> {
        cmd.exec_local()
    }
}

pub struct RemoteCommandInterface {
    config: QemuConfig,
}

impl RemoteCommandInterface {
    pub fn new(config: QemuConfig) -> Self {
        RemoteCommandInterface { config }
    }
}

impl CommandInterface for RemoteCommandInterface {
    fn create_dir_all(&self, path: &RemotePath) -> anyhow::Result<()> {
        let mut mkdir = CommandWrapper::new("mkdir");
        mkdir.arg("-p");
        mkdir.arg(path.base.as_ref());
        self.exec(mkdir)
            .with_context(|| format!("failed to create remote dir at '{}'", path))?;
        Ok(())
    }
    fn remove_dir_all(&self, path: &RemotePath) -> anyhow::Result<()> {
        let mut rm = CommandWrapper::new("rm");
        rm.arg("-rf");
        rm.arg(path.base.as_ref());
        self.exec(rm)
            .with_context(|| format!("failed to remove remote dir at '{}'", path))?;
        Ok(())
    }
    fn copy_to_remote(
        &self,
        local_path: &LocalPath,
        remote_path: &RemotePath,
    ) -> anyhow::Result<()> {
        let mut scp = self.copy_common();
        scp.arg(local_path.as_ref());
        scp.arg(format!("root@localhost:{}", remote_path));
        scp.exec_local().with_context(|| {
            format!(
                "failed to copy file from '{}' (local) to '{}' (remote)",
                local_path,
                remote_path,
            )
        })?;
        Ok(())
    }
    fn copy_from_remote(
        &self,
        remote_path: &RemotePath,
        local_path: &LocalPath,
    ) -> anyhow::Result<()> {
        let mut scp = self.copy_common();
        scp.arg(format!("root@localhost:{}", remote_path));
        scp.arg(local_path.as_ref());
        scp.exec_local().with_context(|| {
            format!(
                "failed to copy file from '{}' (local) to '{}' (remote)",
                remote_path,
                local_path,
            )
        })?;
        Ok(())
    }
    fn exec(&self, cmd: CommandWrapper) -> anyhow::Result<Output> {
        let mut ssh = CommandWrapper::new("ssh");
        ssh.arg("-q");
        ssh.arg("-i").arg(self.config.ssh_private_key_path.clone());
        ssh.arg("-o").arg("StrictHostKeyChecking no");
        ssh.arg("-p").arg(self.config.ssh_port.to_string());
        ssh.arg("root@localhost");
        ssh.arg("-t").arg(format!("{:?}", cmd.internal));
        ssh.exec_local()
            .with_context(|| format!("failed to execute remote command: {:?}", cmd.internal))
    }
}

impl RemoteCommandInterface {
    fn copy_common(&self) -> CommandWrapper {
        let mut scp = CommandWrapper::new("scp");
        scp.arg("-q");
        scp.arg("-i").arg(self.config.ssh_private_key_path.clone());
        scp.arg("-o").arg("StrictHostKeyChecking no");
        // not a typo
        scp.arg("-P").arg(self.config.ssh_port.to_string());
        scp
    }
}
