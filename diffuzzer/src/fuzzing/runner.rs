/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::abstract_fs::trace::{TRACE_FILENAME, Trace};

use crate::abstract_fs::workload::Workload;
use crate::command::CommandInterface;
use crate::config::Config;
use crate::event::EventHandler;
use crate::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};
use crate::save::{save_completed, save_diff, save_testcase};
use anyhow::{Context, Ok};
use hasher::FileDiff;
use log::{debug, info, warn};
use std::fs;
use std::path::Path;
use std::time::Instant;

use super::harness::Harness;
use super::objective::hash::HashObjective;
use super::objective::trace::TraceObjective;
use super::outcome::{Completed, Outcome};

pub struct Runner {
    pub config: Config,

    pub keep_fs: bool,

    pub cmdi: Box<dyn CommandInterface>,
    pub event_handler: EventHandler,

    pub test_dir: RemotePath,
    pub exec_dir: RemotePath,

    pub crashes_path: LocalPath,
    pub accidents_path: LocalPath,

    pub trace_objective: TraceObjective,
    pub hash_objective: HashObjective,

    pub fst_fs_name: String,
    pub snd_fs_name: String,

    pub fst_harness: Harness,
    pub snd_harness: Harness,

    pub stats: Stats,
}

impl Runner {
    pub fn create(
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        config: Config,
        keep_fs: bool,
        cmdi: Box<dyn CommandInterface>,
    ) -> anyhow::Result<Self> {
        let temp_dir = cmdi
            .setup_remote_dir()
            .with_context(|| "failed to setup temp dir")?;

        info!("init runner components");
        let test_dir = temp_dir.clone();
        let exec_dir = temp_dir.join("exec");

        fs::create_dir_all(&crashes_path)?;

        let accidents_path = LocalPath::new(Path::new("./accidents"));
        fs::create_dir_all(&accidents_path)?;

        let fst_fs_name = fst_mount.to_string();
        let snd_fs_name = snd_mount.to_string();

        let fst_fs_dir = RemotePath::new(Path::new("/mnt"))
            .join(fst_fs_name.to_lowercase())
            .join(&config.fs_name);
        let snd_fs_dir = RemotePath::new(Path::new("/mnt"))
            .join(snd_fs_name.to_lowercase())
            .join(&config.fs_name);

        if !config.hashing_enabled {
            warn!("hashing objective is disabled")
        }
        let hash_objective = HashObjective::new(
            fst_fs_dir.clone(),
            snd_fs_dir.clone(),
            fst_mount.get_internal_dirs(),
            snd_mount.get_internal_dirs(),
            config.hashing_enabled,
        );
        let trace_objective = TraceObjective::new();

        let fst_harness = Harness::new(
            fst_mount,
            fst_fs_dir.clone(),
            exec_dir.clone(),
            LocalPath::new_tmp("outcome-1"),
            config.timeout,
        );
        let snd_harness = Harness::new(
            snd_mount,
            snd_fs_dir.clone(),
            exec_dir.clone(),
            LocalPath::new_tmp("outcome-2"),
            config.timeout,
        );

        let event_handler = EventHandler::create(config.qemu.qmp_socket_path.clone())
            .with_context(|| "failed to create event handler")?;

        Ok(Self {
            config,
            keep_fs,

            cmdi,
            event_handler,

            exec_dir,

            test_dir,
            crashes_path,
            accidents_path,

            hash_objective,
            trace_objective,

            fst_fs_name,
            snd_fs_name,
            fst_harness,
            snd_harness,

            stats: Stats::new(),
        })
    }

    pub fn compile_test(&mut self, input: &Workload) -> anyhow::Result<RemotePath> {
        debug!("compile test at '{}'", self.test_dir);
        let binary_path = input
            .compile(self.cmdi.as_ref(), &self.test_dir)
            .with_context(|| "failed to compile test")?;
        Ok(binary_path)
    }

    pub fn run_harness(&mut self, binary_path: &RemotePath) -> anyhow::Result<(Outcome, Outcome)> {
        debug!("run harness at '{}'", binary_path);

        setup_dir(self.cmdi.as_ref(), &self.exec_dir)
            .with_context(|| format!("failed to setup remote exec dir at '{}'", &self.exec_dir))?;
        let fst_hash = if self.hash_objective.enabled {
            Some(&mut self.hash_objective.fst_fs)
        } else {
            None
        };
        let fst_outcome = self
            .fst_harness
            .run(
                self.cmdi.as_ref(),
                binary_path,
                self.keep_fs,
                fst_hash,
                Some(&mut self.event_handler),
            )
            .with_context(|| format!("failed to run first harness '{}'", self.fst_fs_name))?;

        setup_dir(self.cmdi.as_ref(), &self.exec_dir)
            .with_context(|| format!("failed to setup remote exec dir at '{}'", &self.exec_dir))?;
        let snd_hash = if self.hash_objective.enabled {
            Some(&mut self.hash_objective.snd_fs)
        } else {
            None
        };
        let snd_outcome = self
            .snd_harness
            .run(
                self.cmdi.as_ref(),
                binary_path,
                self.keep_fs,
                snd_hash,
                Some(&mut self.event_handler),
            )
            .with_context(|| format!("failed to run second harness '{}'", self.snd_fs_name))?;
        Ok((fst_outcome, snd_outcome))
    }

    pub fn report_crash(
        &mut self,
        input: &Workload,
        dir_name: String,
        binary_path: &RemotePath,
        crash_dir: LocalPath,
        hash_diff: Vec<FileDiff>,
        fst_outcome: &Completed,
        snd_outcome: &Completed,
    ) -> anyhow::Result<()> {
        debug!("report crash '{}'", dir_name);

        let crash_dir = crash_dir.join(dir_name);
        fs::create_dir_all(&crash_dir)
            .with_context(|| format!("failed to create crash directory at '{}'", crash_dir))?;

        save_testcase(self.cmdi.as_ref(), &crash_dir, binary_path, input)?;
        save_completed(&crash_dir, &self.fst_fs_name, fst_outcome)
            .with_context(|| "failed to save first outcome")?;
        save_completed(&crash_dir, &self.snd_fs_name, snd_outcome)
            .with_context(|| "failed to save second outcome")?;

        save_diff(&crash_dir, hash_diff).with_context(|| "failed to save hash differences")?;
        info!("crash saved at '{}'", crash_dir);

        anyhow::Ok(())
    }
}

pub struct Stats {
    pub executions: usize,
    pub crashes: usize,
    pub start: Instant,
    pub last_time_showed: Instant,
}

impl Stats {
    fn new() -> Self {
        Stats {
            executions: 0,
            crashes: 0,
            start: Instant::now(),
            last_time_showed: Instant::now(),
        }
    }
}

pub fn parse_trace(outcome: &Completed) -> anyhow::Result<Trace> {
    let trace = fs::read_to_string(outcome.dir.join(TRACE_FILENAME))?;
    anyhow::Ok(Trace::try_parse(trace).with_context(|| "failed to parse trace")?)
}

pub fn setup_dir(cmdi: &dyn CommandInterface, path: &RemotePath) -> anyhow::Result<()> {
    cmdi.remove_dir_all(path).unwrap_or(());
    cmdi.create_dir_all(path)
}
