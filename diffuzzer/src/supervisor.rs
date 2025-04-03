/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::net::UnixStream,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, JoinHandle, sleep},
    time::Duration,
};

use crate::{
    command::{
        CommandInterface, CommandInterfaceOptions, CommandWrapper, RemoteCommandInterfaceOptions,
        fresh_tcp_port, launch_cmdi,
    },
    config::Config,
    fuzzing::broker::BrokerHandle,
    path::LocalPath,
};
use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use serde_json::{Deserializer, Value};

use crate::config::QemuConfig;

const SNAPSHOT_TAG: &str = "fresh";

/// Controls environment (system) in which tests are executed.
pub trait Supervisor {
    fn load_snapshot(&self) -> anyhow::Result<()>;
    fn save_snapshot(&self) -> anyhow::Result<()>;
    fn reset_events(&mut self) -> anyhow::Result<()>;
    fn had_panic_event(&mut self) -> anyhow::Result<bool>;
}

/// Stub implementation that does nothing
pub struct NativeSupervisor {}

impl NativeSupervisor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Supervisor for NativeSupervisor {
    fn load_snapshot(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn save_snapshot(&self) -> anyhow::Result<()> {
        Ok(())
    }
    fn reset_events(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    fn had_panic_event(&mut self) -> anyhow::Result<bool> {
        Ok(false)
    }
}

pub struct QemuSupervisor {
    options: QemuSupervisorOptions,
    _qemu_thread: JoinHandle<()>,
    event_handler: EventHandler,
    process_id: u32,
    broker: BrokerHandle,
}

impl QemuSupervisor {
    pub fn launch(
        config: &QemuConfig,
        options: QemuSupervisorOptions,
        broker: BrokerHandle,
    ) -> anyhow::Result<Self> {
        let console_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_path)
            .with_context(|| format!("failed to open QEMU log file at '{}'", &config.log_path))?;
        let console_stdio = Stdio::from(console_log);

        let mut launch = Command::new(&config.launch_script);
        launch
            .env("OS_IMAGE", config.os_image.clone())
            .env("SSH_PORT", options.ssh_port.to_string())
            .env("QMP_SOCKET_PATH", options.qmp_socket_path.as_ref())
            .env("MONITOR_SOCKET_PATH", options.monitor_socket_path.as_ref())
            .env("DIRECT_BOOT", config.direct_boot.to_string())
            .env("KERNEL_IMAGE_PATH", &config.kernel_image_path)
            .env("ROOT_DISK_PARTITION", &config.root_disk_partition);
        launch
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(console_stdio);

        let (tx, rx) = mpsc::channel();

        let script = config.launch_script.clone();
        let log_path = config.log_path.clone();
        let builder = thread::Builder::new().name(format!("qemu-process-instance-{}", broker.id()));
        let broker_copy = broker.clone();
        let _qemu_thread = builder
            .spawn(move || {
                match launch
                    .spawn()
                    .with_context(|| format!("failed to run qemu vm from script '{}'", script))
                {
                    Ok(mut child) => {
                        tx.send(child.id()).unwrap();
                        match child.wait() {
                            Ok(status) => {
                                broker
                                    .error(anyhow!(
                                        "qemu finished unexpectedly ({}), check log at '{}'",
                                        status,
                                        log_path
                                    ))
                                    .unwrap();
                            }
                            Err(err) => {
                                broker
                                    .error(anyhow!(
                                        "qemu finished with error, check log at '{}':\n{}",
                                        log_path,
                                        err
                                    ))
                                    .unwrap();
                            }
                        };
                    }
                    Err(err) => broker.error(err).unwrap(),
                };
            })
            .with_context(|| "failed to create qemu thread")?;
        let broker = broker_copy;

        broker.info(format!("wait for VM to init ({}s)", config.boot_wait_time))?;
        sleep(Duration::from_secs(config.boot_wait_time.into()));

        let event_handler = EventHandler::launch(&options.qmp_socket_path, broker.clone())
            .with_context(|| "failed to launch event handler")?;

        let process_id = rx.try_recv()?;
        Ok(Self {
            options,
            _qemu_thread,
            event_handler,
            process_id,
            broker,
        })
    }

    /// Connect to QEMU monitor using QMP protocol
    fn monitor_stream(&self) -> anyhow::Result<UnixStream> {
        UnixStream::connect(&self.options.monitor_socket_path).with_context(|| {
            format!(
                "failed to connect to monitor at '{}'",
                &self.options.monitor_socket_path
            )
        })
    }

    fn check_pid_match(&self) -> bool {
        let mut ps = CommandWrapper::new("ps");
        ps.args(["-p", self.process_id.to_string().as_str(), "-o", "comm="]);
        let p_name: String = ps
            .exec_local(None)
            .and_then(|output| Ok(String::from_utf8(output.stdout).unwrap_or(String::from(""))))
            .unwrap_or(String::from(""));
        p_name.contains("qemu")
    }
}

impl Supervisor for QemuSupervisor {
    fn load_snapshot(&self) -> anyhow::Result<()> {
        self.broker.info("load vm snapshot".into())?;
        let mut stream = self.monitor_stream()?;
        writeln!(stream, "loadvm {}", SNAPSHOT_TAG)?;
        Ok(())
    }

    fn save_snapshot(&self) -> anyhow::Result<()> {
        self.broker.info("save vm snapshot".into())?;
        let mut stream = self.monitor_stream()?;
        writeln!(stream, "savevm {}", SNAPSHOT_TAG)?;
        Ok(())
    }
    fn reset_events(&mut self) -> anyhow::Result<()> {
        self.event_handler.reset()
    }
    fn had_panic_event(&mut self) -> anyhow::Result<bool> {
        self.event_handler.had_panic_event()
    }
}

impl Drop for QemuSupervisor {
    fn drop(&mut self) {
        if !self.check_pid_match() {
            return;
        }
        let mut kill = CommandWrapper::new("kill");
        kill.arg(self.process_id.to_string());
        let _ = kill.exec_local(None);
    }
}

/// Handles events from VM, such as resets, shutdowns and panics.
struct EventHandler {
    rx: Receiver<()>,
}

#[derive(Debug, Deserialize)]
struct ReturnMessage {
    #[serde(rename = "return")]
    _ret: Value,
}

impl EventHandler {
    fn launch(socket_path: &LocalPath, broker: BrokerHandle) -> anyhow::Result<Self> {
        let mut stream = UnixStream::connect(socket_path)
            .with_context(|| format!("failed to connect to unix socket at '{}'", &socket_path))?;
        let mut de = Deserializer::from_reader(stream.try_clone()?);
        Value::deserialize(&mut de).with_context(|| "failed to deserialize response")?;
        stream.write_all(b"{ \"execute\": \"qmp_capabilities\" }")?;
        ReturnMessage::deserialize(&mut de)
            .with_context(|| "failed to deserialize return message")?;

        let (tx, rx): (Sender<()>, Receiver<()>) = mpsc::channel();

        let builder =
            thread::Builder::new().name(format!("event-handler-instance-{}", broker.id()));
        builder
            .spawn(move || {
                loop {
                    let value = Value::deserialize(&mut de)
                        .with_context(|| "failed to deserialize response")
                        .unwrap();
                    if let Value::Object(map) = value {
                        if map.contains_key("event") {
                            tx.send(()).unwrap();
                        }
                    }
                }
            })
            .with_context(|| "failed to spawn event handler thread")?;

        Ok(Self { rx })
    }

    fn had_panic_event(&mut self) -> anyhow::Result<bool> {
        let mut panicked = false;
        loop {
            match self.rx.try_recv() {
                Ok(()) => panicked = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => bail!("event channel disconnected"),
            }
        }
        Ok(panicked)
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        loop {
            match self.rx.try_recv() {
                Ok(()) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => bail!("event channel disconnected"),
            }
        }
        Ok(())
    }
}

pub struct QemuSupervisorOptions {
    pub ssh_port: u16,
    pub qmp_socket_path: LocalPath,
    pub monitor_socket_path: LocalPath,
}

pub enum SupervisorOptions {
    Native,
    Qemu(QemuSupervisorOptions),
}

pub fn launch_supervisor(
    config: &Config,
    options: SupervisorOptions,
    broker: BrokerHandle,
) -> anyhow::Result<Box<dyn Supervisor>> {
    if let SupervisorOptions::Qemu(options) = options {
        Ok(Box::new(
            QemuSupervisor::launch(&config.qemu, options, broker)
                .with_context(|| "failed to launch QEMU supervisor")?,
        ))
    } else {
        Ok(Box::new(NativeSupervisor::new()))
    }
}

pub fn launch_cmdi_and_supervisor(
    no_qemu: bool,
    config: &Config,
    tmp_dir: &LocalPath,
    broker: BrokerHandle,
) -> anyhow::Result<(Box<dyn CommandInterface>, Box<dyn Supervisor>)> {
    let ssh_port =
        fresh_tcp_port().with_context(|| "failed to get fresh port for SSH connection")?;
    let monitor_socket_path = tmp_dir.join("qemu-monitor.sock");
    let qmp_socket_path = tmp_dir.join("qemu-qmp.sock");

    let cmdi_opts = if no_qemu {
        CommandInterfaceOptions::Local
    } else {
        CommandInterfaceOptions::Remote(RemoteCommandInterfaceOptions {
            ssh_port,
            tmp_dir: tmp_dir.clone(),
        })
    };
    let cmdi = launch_cmdi(&config, cmdi_opts);

    let supervisor_opts = if no_qemu {
        SupervisorOptions::Native
    } else {
        SupervisorOptions::Qemu(QemuSupervisorOptions {
            ssh_port,
            monitor_socket_path,
            qmp_socket_path,
        })
    };
    let supervisor = launch_supervisor(&config, supervisor_opts, broker)?;
    Ok((cmdi, supervisor))
}
