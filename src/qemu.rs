use std::{collections::HashMap, process::Command};

use anyhow::Context;
use log::{error, info};

use crate::config::QemuConfig;

pub fn run(config: QemuConfig) {
    let mut qemu = Command::new(config.launch_options.cmd.clone());
    let opts = config.launch_options;
    qemu.arg("-machine").arg(map_to_args(opts.machine));
    qemu.arg("-cpu").arg(opts.cpu);
    qemu.arg("-smp").arg(map_to_args(opts.smp));
    qemu.arg("-m").arg(opts.memory);
    qemu.arg("-monitor").arg(format!(
        "tcp::{},{}",
        opts.monitor.tcp_port,
        opts.monitor.extra.join(",")
    ));
    qemu.arg("-device")
        .arg(format!("{},netdev={}", opts.netdev.driver, opts.netdev.id,));
    qemu.arg("-netdev").arg(format!(
        "user,id={},hostfwd=tcp::{}-:22",
        opts.netdev.id, opts.netdev.ssh_host_forward_port
    ));
    qemu.arg("-drive").arg(map_to_args(opts.drive));
    qemu.args(opts.extra);
    info!("Running QEMU: \n{:?}", qemu);
    let mut qemu_process = qemu
        .spawn()
        .with_context(|| "failed to start QEMU")
        .unwrap();
    match qemu_process.wait() {
        Ok(status) => info!("stopped with status: {}", status),
        Err(err) => error!("stopped with error: {}", err),
    }
}

fn map_to_args(map: HashMap<String, String>) -> String {
    let pairs: Vec<String> = map.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    pairs.join(",")
}
