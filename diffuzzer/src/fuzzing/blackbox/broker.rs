/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    fs,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Instant,
};

use anyhow::{Context, bail};
use log::info;

use crate::{
    config::Config, fuzzing::fuzzer::Fuzzer, mount::FileSystemMount, path::LocalPath,
    supervisor::launch_cmdi_and_supervisor,
};

use super::fuzzer::BlackBoxFuzzer;

pub enum BrokerMessage {
    Error {
        id: u8,
        err: anyhow::Error,
    },
    Stats {
        id: u8,
        executions: u64,
        crashes: u64,
    },
}

pub enum InstanceMessage {
    Run { test_count: Option<u64> },
}

pub struct Instance {
    id: u8,
    _handle: JoinHandle<()>,
    tx: Sender<InstanceMessage>,
    executions: u64,
    crashes: u64,
}

pub struct BlackBoxBroker {
    instances: Vec<Instance>,
    rx: Receiver<BrokerMessage>,
    start: Instant,
}

impl BlackBoxBroker {
    pub fn create(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        no_qemu: bool,
        instances_n: u8,
    ) -> anyhow::Result<Self> {
        if instances_n == 0 || no_qemu && instances_n > 1 {
            bail!("invalid number of instances ({})", instances_n);
        }
        let mut instances = Vec::new();
        let (broker_tx, broker_rx) = mpsc::channel();
        for id in 0..instances_n {
            let broker_tx = broker_tx.clone();
            let (instance_tx, instance_rx) = mpsc::channel();
            let config = config.clone();
            let crashes_path = crashes_path.clone();

            let builder = thread::Builder::new();
            let name = format!("instance-{}", id);
            let handle = builder
                .name(name.clone())
                .spawn(move || {
                    let local_tmp_dir = LocalPath::new_tmp(&name);
                    fs::remove_dir(local_tmp_dir.as_ref()).unwrap_or(());
                    match fs::create_dir_all(local_tmp_dir.as_ref()).with_context(|| {
                        format!(
                            "failed to create local temporary directory for instance {} at '{}'",
                            id, local_tmp_dir
                        )
                    }) {
                        Err(err) => broker_tx
                            .send(BrokerMessage::Error { id, err })
                            .expect("failed to send broker message"),
                        Ok(_) => {
                            match launch_cmdi_and_supervisor(no_qemu, &config, &local_tmp_dir) {
                                Err(err) => broker_tx
                                    .send(BrokerMessage::Error { id, err })
                                    .expect("failed to send broker message"),
                                Ok((cmdi, supervisor)) => {
                                    match BlackBoxFuzzer::create(
                                        id,
                                        config.clone(),
                                        fst_mount,
                                        snd_mount,
                                        crashes_path.clone(),
                                        cmdi,
                                        supervisor,
                                        local_tmp_dir,
                                        broker_tx.clone(),
                                    )
                                    .with_context(|| {
                                        format!("failed to launch fuzzer instance {}", id)
                                    }) {
                                        Err(err) => broker_tx
                                            .send(BrokerMessage::Error { id, err })
                                            .expect("failed to send broker message"),
                                        Ok(mut instance) => {
                                            let InstanceMessage::Run { test_count } = instance_rx
                                                .recv()
                                                .expect("failed to receive instance message");
                                            match instance.run(test_count) {
                                                Ok(_) => {}
                                                Err(err) => broker_tx
                                                    .send(BrokerMessage::Error { id, err })
                                                    .expect("failed to send broker message"),
                                            };
                                        }
                                    };
                                }
                            };
                        }
                    };
                })
                .with_context(|| format!("failed to create instance {}", id))?;
            instances.push(Instance {
                id,
                _handle: handle,
                tx: instance_tx,
                executions: 0,
                crashes: 0,
            });
        }
        Ok(Self {
            instances,
            rx: broker_rx,
            start: Instant::now(),
        })
    }

    pub fn run(&mut self, test_count: Option<u64>) -> anyhow::Result<()> {
        self.start = Instant::now();
        for i in self.instances.iter() {
            info!("run fuzzer instance {}", i.id);
            i.tx.send(InstanceMessage::Run { test_count })
                .with_context(|| "failed to run instance")?;
        }
        loop {
            match self
                .rx
                .recv()
                .with_context(|| "failed to receive broker message")?
            {
                BrokerMessage::Error { id, err } => {
                    return Err(err.context(format!("error inside instance {}", id)));
                }
                BrokerMessage::Stats {
                    id,
                    executions,
                    crashes,
                } => {
                    let instance = self
                        .instances
                        .get_mut(id as usize)
                        .with_context(|| format!("failed to get instance {}", id))?;
                    instance.executions = executions;
                    instance.crashes = crashes;
                    let global_executions =
                        self.instances.iter().fold(0, |acc, i| acc + i.executions);
                    let global_crashes = self.instances.iter().fold(0, |acc, i| acc + i.crashes);
                    let secs = self.start.elapsed().as_secs();

                    let instance_stats = format!(
                        "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
                        crashes,
                        executions,
                        (executions as f64) / (secs as f64),
                        secs / (60 * 60),
                        (secs / (60)) % 60,
                        secs % 60,
                    );
                    let global_stats = format!(
                        "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
                        global_crashes,
                        global_executions,
                        (global_executions as f64) / (secs as f64),
                        secs / (60 * 60),
                        (secs / (60)) % 60,
                        secs % 60,
                    );
                    info!("{}", global_stats);
                    info!("{} (instance {})", instance_stats, id);
                }
            }
        }
    }
}
