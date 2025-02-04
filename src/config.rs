use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::abstract_fs::{mutator::MutationWeights, operation::OperationWeights};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub greybox: GreyboxConfig,
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
    pub max_workload_length: u16,
    pub fs_name: String,
    pub hashing_enabled: bool,
    pub heartbeat_interval: u16,
    pub timeout: u8,
    pub qemu_config: QemuConfig,
}

#[derive(Serialize, Deserialize)]
pub struct GreyboxConfig {
    pub max_mutations: u16,
    pub save_corpus: bool,
}

#[derive(Serialize, Deserialize)]
pub struct QemuConfig {
    /// Options used to launch QEMU
    pub launch_options: QemuLaunchOptions,
    /// Private key used to connect to VM instance using SSH
    pub ssh_private_key_path: String,
}

/// [QEMU documentation](https://www.qemu.org/docs/master/system/invocation.html)
#[derive(Serialize, Deserialize)]
pub struct QemuLaunchOptions {
    /// QEMU command to run
    pub cmd: String,
    /// Select the emulated machine by name: `-machine ...`
    pub machine: HashMap<String, String>,
    /// CPU model: `-cpu ...`
    pub cpu: String,
    /// CPU topology hierarchy: `-smp ...`
    pub smp: HashMap<String, String>,
    /// Memory available for instance: `-m ...`
    pub memory: String,
    /// QMP monitor: `-monitor ...`
    pub monitor: MonitorOptions,
    /// Network device for SSH: `-device ...` + `-netdev user,...`
    pub netdev: NetDeviceOptions,
    /// Drive with OS image
    pub drive: HashMap<String, String>,
    /// Extra options
    pub extra: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MonitorOptions {
    /// Host TCP port for monitor connection
    pub tcp_port: String,
    /// Extra options
    pub extra: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct NetDeviceOptions {
    /// Device driver
    pub driver: String,
    /// Unique device identifier
    pub id: String,
    /// Host TCP port for SSH connection to VM
    pub ssh_host_forward_port: String,
}
