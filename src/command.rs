use std::{
    ffi::OsStr,
    fs,
    path::Path,
    process::{Command, Output},
};

use anyhow::{bail, Context, Ok};
use log::info;

use crate::{
    config::QemuConfig,
    path::{LocalPath, RemotePath},
};

const EXECUTOR_SOURCE_DIR: &str = "./executor";
const MAKEFILE_NAME: &str = "makefile";
const EXECUTOR_H_NAME: &str = "executor.h";
const EXECUTOR_CPP_NAME: &str = "executor.cpp";
const TEST_NAME: &str = "test.c";

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
    fn copy_dir_from_remote(
        &self,
        remote_path: &RemotePath,
        local_path: &LocalPath,
    ) -> anyhow::Result<()>;
    fn write(&self, path: &RemotePath, contents: &[u8]) -> anyhow::Result<()>;
    fn read_to_string(&self, path: &RemotePath) -> anyhow::Result<String>;

    fn exec(&self, cmd: CommandWrapper) -> anyhow::Result<Output>;
    fn exec_in_dir(&self, cmd: CommandWrapper, dir: &RemotePath) -> anyhow::Result<Output>;

    fn setup_remote_dir(&self) -> anyhow::Result<RemotePath> {
        let remote_dir = RemotePath::new_tmp("remote");

        info!(
            "setting up remote directory at '{}'",
            remote_dir.base.display()
        );
        self.remove_dir_all(&remote_dir).unwrap_or(());
        self.create_dir_all(&remote_dir).with_context(|| {
            format!(
                "failed to create remote directory at '{}'",
                remote_dir.base.display()
            )
        })?;

        info!(
            "copying executor to remote directory '{}'",
            remote_dir.base.display()
        );
        let executor_dir = LocalPath::new(&Path::new(EXECUTOR_SOURCE_DIR));
        self.copy_to_remote(
            &executor_dir.join(MAKEFILE_NAME),
            &remote_dir.join(MAKEFILE_NAME),
        )?;
        self.copy_to_remote(
            &executor_dir.join(EXECUTOR_H_NAME),
            &remote_dir.join(EXECUTOR_H_NAME),
        )?;
        self.copy_to_remote(
            &executor_dir.join(EXECUTOR_CPP_NAME),
            &remote_dir.join(EXECUTOR_CPP_NAME),
        )?;
        self.copy_to_remote(
            &executor_dir.join(EXECUTOR_CPP_NAME),
            &remote_dir.join(EXECUTOR_CPP_NAME),
        )?;
        self.copy_to_remote(&executor_dir.join(TEST_NAME), &remote_dir.join(TEST_NAME))?;

        let mut make = CommandWrapper::new("make");
        make.arg("-C").arg(executor_dir.as_ref());
        self.exec(make)
            .with_context(|| "failed to make test binary")?;

        Ok(remote_dir)
    }
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
                local_path, remote_path,
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
                remote_path, local_path,
            )
        })?;
        Ok(())
    }
    fn copy_dir_from_remote(
        &self,
        remote_path: &RemotePath,
        local_path: &LocalPath,
    ) -> anyhow::Result<()> {
        let mut cp = CommandWrapper::new("cp");
        cp.arg("-r")
            .arg(remote_path.base.as_ref())
            .arg(local_path.as_ref());
        cp.exec_local().with_context(|| {
            format!(
                "failed to copy local directory from '{}' to '{}'",
                remote_path, local_path
            )
        })?;
        Ok(())
    }
    fn write(&self, path: &RemotePath, contents: &[u8]) -> anyhow::Result<()> {
        fs::write(path.base.as_ref(), contents)
            .with_context(|| format!("failed to write local file '{}'", path))
    }
    fn read_to_string(&self, path: &RemotePath) -> anyhow::Result<String> {
        fs::read_to_string(path.base.as_ref())
            .with_context(|| format!("failed to read local file '{}'", path))
    }

    fn exec(&self, cmd: CommandWrapper) -> anyhow::Result<Output> {
        cmd.exec_local()
    }
    fn exec_in_dir(&self, cmd: CommandWrapper, dir: &RemotePath) -> anyhow::Result<Output> {
        let mut cmd = cmd;
        cmd.internal.current_dir(dir.base.as_ref());
        cmd.exec_local()
    }
}

pub struct RemoteCommandInterface {
    config: QemuConfig,
    tmp_file: LocalPath,
}

impl RemoteCommandInterface {
    pub fn new(config: QemuConfig) -> Self {
        RemoteCommandInterface {
            config,
            tmp_file: LocalPath::new_tmp("ssh-tmp"),
        }
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
                local_path, remote_path,
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
                remote_path, local_path,
            )
        })?;
        Ok(())
    }
    fn copy_dir_from_remote(
        &self,
        remote_path: &RemotePath,
        local_path: &LocalPath,
    ) -> anyhow::Result<()> {
        let mut scp = self.copy_common();
        scp.arg("-r");
        scp.arg(format!("root@localhost:{}", remote_path));
        scp.arg(local_path.as_ref());
        scp.exec_local().with_context(|| {
            format!(
                "failed to copy file from '{}' (local) to '{}' (remote)",
                remote_path, local_path,
            )
        })?;
        Ok(())
    }
    fn write(&self, path: &RemotePath, contents: &[u8]) -> anyhow::Result<()> {
        fs::write(self.tmp_file.as_ref(), contents)
            .with_context(|| format!("failed to write to temporary file at '{}'", self.tmp_file))?;
        self.copy_to_remote(&self.tmp_file, path)?;
        fs::remove_file(self.tmp_file.as_ref())
            .with_context(|| format!("failed to remove temporary file at '{}'", self.tmp_file))
    }
    fn read_to_string(&self, path: &RemotePath) -> anyhow::Result<String> {
        self.copy_from_remote(path, &self.tmp_file)?;
        let s = fs::read_to_string(&self.tmp_file)
            .with_context(|| format!("failed to read from temprary file at '{}'", self.tmp_file))?;
        fs::remove_file(self.tmp_file.as_ref())
            .with_context(|| format!("failed to remove temporary file at '{}'", self.tmp_file))?;
        Ok(s)
    }

    fn exec(&self, cmd: CommandWrapper) -> anyhow::Result<Output> {
        let mut ssh = self.exec_common();
        ssh.arg("-t").arg(format!("{:?}", cmd.internal));
        ssh.exec_local()
            .with_context(|| format!("failed to execute remote command: {:?}", cmd.internal))
    }
    fn exec_in_dir(&self, cmd: CommandWrapper, dir: &RemotePath) -> anyhow::Result<Output> {
        let mut ssh = self.exec_common();
        ssh.arg("-t")
            .arg("cd")
            .arg(dir.base.as_ref())
            .arg("&&")
            .arg(format!("{:?}", cmd.internal));
        ssh.exec_local().with_context(|| {
            format!(
                "failed to execute remote command in directory '{}': {:?}",
                dir, cmd.internal
            )
        })
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
    fn exec_common(&self) -> CommandWrapper {
        let mut ssh = CommandWrapper::new("ssh");
        ssh.arg("-q");
        ssh.arg("-i").arg(self.config.ssh_private_key_path.clone());
        ssh.arg("-o").arg("StrictHostKeyChecking no");
        ssh.arg("-p").arg(self.config.ssh_port.to_string());
        ssh.arg("root@localhost");
        ssh
    }
}
