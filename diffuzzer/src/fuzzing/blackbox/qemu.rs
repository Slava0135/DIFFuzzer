/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, Ok};
use log::{debug, info};
use rand::prelude::StdRng;
use rand::SeedableRng;
use std::io::Write;
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::abstract_fs::generator::generate_new;
use crate::command::RemoteCommandInterface;
use crate::config::Config;

use crate::fuzzing::fuzzer::Fuzzer;
use crate::fuzzing::runner::{parse_trace, Runner};
use crate::mount::mount::FileSystemMount;
use crate::path::LocalPath;

const SNAPSHOT_TAG: &str = "FRESH";

pub struct QemuBlackBoxFuzzer {
    runner: Runner,
    rng: StdRng,
    qemu_process: Child,
    monitor_stream: TcpStream,
}

impl QemuBlackBoxFuzzer {
    pub fn new(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
    ) -> Self {
        let mut launch = Command::new(&config.qemu.launch_script);
        launch
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let qemu_process = launch.spawn().expect(&format!(
            "failed to run qemu vm from script '{}'",
            &config.qemu.launch_script
        ));

        info!("wait for VM to init");
        sleep(Duration::from_secs(10));

        let runner = Runner::new(
            fst_mount,
            snd_mount,
            crashes_path,
            config.clone(),
            false,
            Box::new(RemoteCommandInterface::new(config.qemu.clone())),
        );

        let addr = format!("localhost:{}", config.qemu.monitor_port);
        let mut monitor_stream = TcpStream::connect(addr.clone()).expect(&format!(
            "failed to connect to qemu monitor at address '{}'",
            addr
        ));
        monitor_stream
            .set_nodelay(true)
            .expect("failed to call nodelay");
        monitor_stream
            .write_all(format!("savevm {}", SNAPSHOT_TAG).as_bytes())
            .expect("failed to save vm snapshot");

        Self {
            runner,
            rng: StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
            qemu_process,
            monitor_stream,
        }
    }
}

impl Fuzzer for QemuBlackBoxFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        debug!("generate input");
        let input = generate_new(
            &mut self.rng,
            self.runner.config.max_workload_length.into(),
            &self.runner.config.operation_weights,
        );

        let binary_path = self.runner().compile_test(&input)?;

        debug!("run harness at '{}'", binary_path);

        let (fst_outcome, snd_outcome) = self.runner().run_harness(&binary_path)?;

        let fst_trace =
            parse_trace(&fst_outcome).with_context(|| format!("failed to parse first trace"))?;
        let snd_trace =
            parse_trace(&snd_outcome).with_context(|| format!("failed to parse second trace"))?;

        if self.detect_errors(
            &input,
            &binary_path,
            &fst_trace,
            &snd_trace,
            &fst_outcome,
            &snd_outcome,
        )? {
            return Ok(());
        }

        self.do_objective(
            &input,
            &binary_path,
            &fst_trace,
            &snd_trace,
            &fst_outcome,
            &snd_outcome,
        )?;

        Ok(())
    }

    fn show_stats(&mut self) {
        self.runner.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.runner.stats.start);
        let secs = since_start.as_secs();
        info!(
            "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.runner.stats.crashes,
            self.runner.stats.executions,
            (self.runner.stats.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        );
    }

    fn runner(&mut self) -> &mut Runner {
        &mut self.runner
    }
}
