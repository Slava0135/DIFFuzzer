use std::{
    process::{Command, Stdio},
    thread::{self, sleep},
    time::Duration,
};

use anyhow::Context;
use log::{error, info};

use crate::config::QemuConfig;

pub fn launch(config: &QemuConfig) -> anyhow::Result<()> {
    let mut launch = Command::new(&config.launch_script);
    launch
        .env("OS_IMAGE", config.os_image.clone())
        .env("MONITOR_PORT", config.monitor_port.to_string())
        .env("SSH_PORT", config.ssh_port.to_string());
    launch
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let script = config.launch_script.clone();
    thread::spawn(move || {
        match launch
            .spawn()
            .with_context(|| format!("failed to run qemu vm from script '{}'", script))
        {
            Ok(mut child) => match child.wait() {
                Ok(status) => {
                    error!("qemu finished unexpectedly ({})", status);
                }
                Err(err) => {
                    error!("qemu finished with error:\n{}", err)
                }
            },
            Err(err) => error!("{}", err),
        };
    });

    info!("wait for VM to init ({}s)", config.boot_wait_time);
    sleep(Duration::from_secs(config.boot_wait_time.into()));

    Ok(())
}
