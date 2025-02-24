use std::{
    fs::OpenOptions,
    process::{Command, Stdio},
    thread::{self, sleep},
    time::Duration,
};

use anyhow::Context;
use log::{error, info};

use crate::config::QemuConfig;

pub fn launch(config: &QemuConfig) -> anyhow::Result<()> {
    let console_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)
        .with_context(|| format!("failed to open QEMU log file at '{}'", &config.log_path))?;
    let console_stdio = Stdio::from(console_log);

    let mut launch = Command::new(&config.launch_script);
    launch
        .env("OS_IMAGE", config.os_image.clone())
        .env("MONITOR_PORT", config.monitor_port.to_string())
        .env("SSH_PORT", config.ssh_port.to_string())
        .env("QMP_SOCKET_PATH", config.qmp_socket_path.clone());
    launch
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(console_stdio);

    let script = config.launch_script.clone();
    let log_path = config.log_path.clone();
    thread::spawn(move || {
        match launch
            .spawn()
            .with_context(|| format!("failed to run qemu vm from script '{}'", script))
        {
            Ok(mut child) => match child.wait() {
                Ok(status) => {
                    error!(
                        "qemu finished unexpectedly ({}), check log at '{}'",
                        status, log_path
                    );
                }
                Err(err) => {
                    error!(
                        "qemu finished with error, check log at '{}':\n{}",
                        log_path, err
                    )
                }
            },
            Err(err) => error!("{}", err),
        };
    });

    info!("wait for VM to init ({}s)", config.boot_wait_time);
    sleep(Duration::from_secs(config.boot_wait_time.into()));

    Ok(())
}
