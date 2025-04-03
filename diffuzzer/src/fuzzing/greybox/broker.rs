/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Instant,
};

use anyhow::{Context, bail};
use log::{info, warn};

use crate::{
    config::Config,
    fuzzing::{
        broker::{BrokerHandle, BrokerMessage, GreyBoxStats, InstanceMessage},
        fuzzer::Fuzzer,
    },
    mount::FileSystemMount,
    path::LocalPath,
    supervisor::launch_cmdi_and_supervisor,
};

use super::fuzzer::GreyBoxFuzzer;

struct GreyBoxInstance {
    _handle: JoinHandle<()>,
    tx: Sender<InstanceMessage>,
    stats: GreyBoxStats,
}

pub struct GreyBoxBroker {
    instances: Vec<GreyBoxInstance>,
    rx: Receiver<BrokerMessage>,
    start: Instant,
}

impl GreyBoxBroker {
    pub fn create(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        corpus_path: Option<String>,
        no_qemu: bool,
        instances_n: u8,
    ) -> anyhow::Result<Self> {
        if instances_n == 0 || no_qemu && instances_n > 1 {
            bail!("invalid number of instances ({})", instances_n);
        }
        let mut instances = Vec::new();
        let (broker_tx, broker_rx) = mpsc::channel();
        for id in 0..instances_n {
            let broker = BrokerHandle::Full {
                id,
                tx: broker_tx.clone(),
            };
            let (instance_tx, instance_rx) = mpsc::channel();
            let config = config.clone();
            let crashes_path = crashes_path.clone();
            let corpus_path = corpus_path.clone();

            let builder = thread::Builder::new();
            let name = format!("instance-{}", id);
            let handle = builder
                .name(name.clone())
                .spawn(move || {
                    match LocalPath::create_new_tmp(&name) {
                        Err(err) => broker.error(err).unwrap(),
                        Ok(local_tmp_dir) => {
                            match launch_cmdi_and_supervisor(
                                no_qemu,
                                &config,
                                &local_tmp_dir,
                                broker.clone(),
                            ) {
                                Err(err) => broker.error(err).unwrap(),
                                Ok((cmdi, supervisor)) => {
                                    match GreyBoxFuzzer::create(
                                        config.clone(),
                                        fst_mount,
                                        snd_mount,
                                        crashes_path.clone(),
                                        corpus_path,
                                        cmdi,
                                        supervisor,
                                        local_tmp_dir,
                                        broker.clone(),
                                    )
                                    .with_context(|| {
                                        format!("failed to launch fuzzer instance {}", id)
                                    }) {
                                        Err(err) => broker.error(err).unwrap(),
                                        Ok(mut instance) => {
                                            broker.info("fuzzer is ready".into()).unwrap();
                                            let InstanceMessage::Run { test_count } = instance_rx
                                                .recv()
                                                .expect("failed to receive instance message");
                                            broker.info("run fuzzer".into()).unwrap();
                                            match instance.run(test_count) {
                                                Ok(_) => {}
                                                Err(err) => broker.error(err).unwrap(),
                                            };
                                        }
                                    };
                                }
                            };
                        }
                    };
                })
                .with_context(|| format!("failed to create instance {}", id))?;
            instances.push(GreyBoxInstance {
                _handle: handle,
                tx: instance_tx,
                stats: Default::default(),
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
                BrokerMessage::BlackBoxStats { .. } => {
                    panic!("grey box broker received black box stats")
                }
                BrokerMessage::GreyBoxStats { id, stats } => {
                    let instance = self
                        .instances
                        .get_mut(id as usize)
                        .with_context(|| format!("failed to get instance {}", id))?;
                    instance.stats = stats.clone();
                    let aggregated =
                        GreyBoxStats::aggregate(self.instances.iter().map(|i| &i.stats).collect());

                    info!("{}", aggregated.display(&self.start));
                    info!("{} (instance {})", stats.display(&self.start), id);
                }
                BrokerMessage::Info { id, msg } => {
                    info!("{} (instance {})", msg, id);
                }
                BrokerMessage::Warn { id, msg } => {
                    warn!("{} (instance {})", msg, id);
                }
            }
        }
    }
}
