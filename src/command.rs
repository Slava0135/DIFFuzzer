use std::{
    ffi::OsStr,
    fs,
    path::Path,
    process::{Command, Output},
};

use anyhow::{bail, Context, Ok};

use crate::config::QemuConfig;

pub trait CommandInterface {
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()>;
    fn copy_to_guest(&self, host_path: &Path, guest_path: &Path) -> anyhow::Result<()>;
    fn copy_to_host(&self, guest_path: &Path, host_path: &Path) -> anyhow::Result<()>;

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
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("failed to create local dir at '{}'", path.display()))
    }
    fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove local dir at '{}'", path.display()))
    }
    fn copy_to_guest(&self, host_path: &Path, guest_path: &Path) -> anyhow::Result<()> {
        fs::copy(host_path, guest_path).with_context(|| {
            format!(
                "failed to copy local file from '{}' to '{}'",
                host_path.display(),
                guest_path.display(),
            )
        })?;
        Ok(())
    }
    fn copy_to_host(&self, guest_path: &Path, host_path: &Path) -> anyhow::Result<()> {
        fs::copy(guest_path, host_path).with_context(|| {
            format!(
                "failed to copy local file from '{}' to '{}'",
                guest_path.display(),
                host_path.display(),
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
    fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        let mut mkdir = CommandWrapper::new("mkdir");
        mkdir.arg("-p");
        mkdir.arg(path);
        self.exec(mkdir)
            .with_context(|| format!("failed to create remote dir at '{}'", path.display()))?;
        Ok(())
    }
    fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        let mut rm = CommandWrapper::new("rm");
        rm.arg("-rf");
        rm.arg(path);
        self.exec(rm)
            .with_context(|| format!("failed to remove remote dir at '{}'", path.display()))?;
        Ok(())
    }
    fn copy_to_guest(&self, host_path: &Path, guest_path: &Path) -> anyhow::Result<()> {
        let mut scp = self.copy_common();
        scp.arg(host_path);
        scp.arg(format!("root@localhost:{}", guest_path.display()));
        scp.exec_local()?;
        Ok(())
    }
    fn copy_to_host(&self, guest_path: &Path, host_path: &Path) -> anyhow::Result<()> {
        let mut scp = self.copy_common();
        scp.arg(format!("root@localhost:{}", guest_path.display()));
        scp.arg(host_path);
        scp.exec_local()?;
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
