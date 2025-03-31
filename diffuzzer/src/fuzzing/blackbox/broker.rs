/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    fs,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use anyhow::{Context, bail};

use crate::{
    config::Config, fuzzing::fuzzer::Fuzzer, mount::FileSystemMount, path::LocalPath,
    supervisor::launch_cmdi_and_supervisor,
};

use super::fuzzer::BlackBoxFuzzer;

pub enum BrokerMessage {
    Error { id: u8, err: anyhow::Error },
}

pub enum InstanceMessage {
    Run { test_count: Option<u64> },
}

pub struct Instance {
    handle: JoinHandle<()>,
    tx: Sender<InstanceMessage>,
}

pub struct BlackBoxBroker {
    instances: Vec<Instance>,
    rx: Receiver<BrokerMessage>,
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
        for id in 1..=instances_n {
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
                handle,
                tx: instance_tx,
            });
        }
        Ok(Self {
            instances,
            rx: broker_rx,
        })
    }

    pub fn run(&mut self, test_count: Option<u64>) -> anyhow::Result<()> {
        Ok(())
    }
}
