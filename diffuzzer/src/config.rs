/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use serde::{Deserialize, Serialize};

use crate::abstract_fs::{mutator::MutationWeights, operation::OperationWeights};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub greybox: GreyboxConfig,
    pub operation_weights: OperationWeights,
    pub mutation_weights: MutationWeights,
    pub max_workload_length: u16,
    /// Filesystem name that is used for mountpoint path
    pub fs_name: String,
    /// Interval after which, if nothing happens, log is updated
    pub heartbeat_interval: u16,
    /// Timeout for executing a single test
    pub timeout: u8,
    pub qemu: QemuConfig,
    pub dash: DashConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GreyboxConfig {
    pub max_mutations: u16,
    /// If enabled corpus testcases will be also saved in separate directory
    pub save_corpus: bool,
    pub scheduler: Scheduler,
    /// Constant used for some schedulers
    pub m_constant: u64,
}

/// See [`crate::fuzzing::greybox::schedule`]
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum Scheduler {
    Queue,
    Fast,
}

/// [QEMU documentation](https://www.qemu.org/docs/master/system/invocation.html)
#[derive(Serialize, Deserialize, Clone)]
pub struct QemuConfig {
    /// Path to VM launch script
    pub launch_script: String,
    /// Private key used to connect to VM instance using SSH
    pub ssh_private_key_path: String,
    /// Path to OS image
    pub os_image: String,
    /// Time to wait until OS is considered booted
    pub boot_wait_time: u8,
    /// Path to QEMU log file
    pub log_path: String,
    /// Run QEMU direct boot with custom kernel and command line arguments
    /// This is required for fuzzing with KASAN
    pub direct_boot: bool,
    /// Path to kernel bzImage (direct boot)
    pub kernel_image_path: String,
    /// Root disk partition (direct boot)
    pub root_disk_partition: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DashConfig {
    pub enabled: bool,
    pub debug_binary_path: String,
    pub release_binary_path: String,
}
