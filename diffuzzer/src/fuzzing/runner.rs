/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::abstract_fs::trace::{TRACE_FILENAME, Trace};

use crate::abstract_fs::workload::Workload;
use crate::command::CommandInterface;
use crate::config::Config;
use crate::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};
use crate::save::{save_completed, save_diff, save_reason, save_testcase};
use crate::supervisor::Supervisor;
use anyhow::{Context, Ok};
use dash::FileDiff;
use log::{debug, info};
use std::fs;
use std::path::Path;
use std::time::Instant;

use super::harness::Harness;
use super::objective::dash::DashObjective;
use super::objective::trace::TraceObjective;
use super::outcome::{Completed, Outcome};

pub struct Runner {
    pub config: Config,

    pub keep_fs: bool,

    pub cmdi: Box<dyn CommandInterface>,
    pub supervisor: Box<dyn Supervisor>,

    pub test_dir: RemotePath,
    pub exec_dir: RemotePath,

    pub crashes_path: LocalPath,
    pub accidents_path: LocalPath,

    pub trace_objective: TraceObjective,
    pub dash_objective: DashObjective,

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
        supervisor: Box<dyn Supervisor>,
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

        let dash_objective = DashObjective::create(
            cmdi.as_ref(),
            fst_fs_dir.clone(),
            snd_fs_dir.clone(),
            fst_mount.get_internal_dirs(),
            snd_mount.get_internal_dirs(),
            &config,
        )
        .with_context(|| "failed to create Dash objective")?;
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

        let runner = Self {
            config,
            keep_fs,

            cmdi,
            supervisor,

            exec_dir,

            test_dir,
            crashes_path,
            accidents_path,

            dash_objective,
            trace_objective,

            fst_fs_name,
            snd_fs_name,
            fst_harness,
            snd_harness,

            stats: Stats::new(),
        };

        runner
            .supervisor
            .save_snapshot()
            .with_context(|| "failed to save snapshot")?;

        Ok(runner)
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
        let fst_outcome = self
            .fst_harness
            .run(
                self.cmdi.as_ref(),
                binary_path,
                self.keep_fs,
                self.supervisor.as_mut(),
                |cmdi| {
                    self.dash_objective
                        .calculate_fst(cmdi)
                        .with_context(|| "Failed on Dash calculating")
                        .unwrap()
                },
            )
            .with_context(|| format!("failed to run first harness '{}'", self.fst_fs_name))?;
        match fst_outcome {
            Outcome::Panicked => {
                self.supervisor
                    .load_snapshot()
                    .with_context(|| "failed to load snapshot")?;
                return Ok((fst_outcome, Outcome::Skipped));
            }
            Outcome::TimedOut => return Ok((fst_outcome, Outcome::Skipped)),
            _ => {}
        }

        setup_dir(self.cmdi.as_ref(), &self.exec_dir)
            .with_context(|| format!("failed to setup remote exec dir at '{}'", &self.exec_dir))?;
        let snd_outcome = self
            .snd_harness
            .run(
                self.cmdi.as_ref(),
                binary_path,
                self.keep_fs,
                self.supervisor.as_mut(),
                |cmdi| {
                    self.dash_objective
                        .calculate_snd(cmdi)
                        .with_context(|| "Failed on Dash calculating")
                        .unwrap()
                },
            )
            .with_context(|| format!("failed to run second harness '{}'", self.snd_fs_name))?;

        if let Outcome::Panicked = snd_outcome {
            self.supervisor
                .load_snapshot()
                .with_context(|| "failed to load snapshot")?;
        }

        Ok((fst_outcome, snd_outcome))
    }

    pub fn report_diff(
        &mut self,
        input: &Workload,
        dir_name: String,
        binary_path: &RemotePath,
        crash_dir: LocalPath,
        hash_diff: Vec<FileDiff>,
        fst_outcome: &Completed,
        snd_outcome: &Completed,
        reason: String,
    ) -> anyhow::Result<()> {
        debug!("report diff '{}'", dir_name);

        let crash_dir = crash_dir.join(dir_name);
        fs::create_dir_all(&crash_dir)
            .with_context(|| format!("failed to create crash directory at '{}'", crash_dir))?;

        save_testcase(self.cmdi.as_ref(), &crash_dir, Some(binary_path), input)
            .with_context(|| "failed to save testcase")?;
        save_completed(&crash_dir, &self.fst_fs_name, fst_outcome)
            .with_context(|| "failed to save first outcome")?;
        save_completed(&crash_dir, &self.snd_fs_name, snd_outcome)
            .with_context(|| "failed to save second outcome")?;

        save_diff(&crash_dir, hash_diff).with_context(|| "failed to save hash differences")?;
        save_reason(&crash_dir, reason).with_context(|| "failed to save reason")?;

        info!("diff saved at '{}'", crash_dir);

        Ok(())
    }

    pub fn report_crash(
        &mut self,
        input: &Workload,
        dir_name: String,
        crash_dir: LocalPath,
        reason: String,
    ) -> anyhow::Result<()> {
        debug!("report panic '{}'", dir_name);

        let crash_dir = crash_dir.join(dir_name);
        fs::create_dir_all(&crash_dir)
            .with_context(|| format!("failed to create crash directory at '{}'", crash_dir))?;

        save_testcase(self.cmdi.as_ref(), &crash_dir, None, input)
            .with_context(|| "failed to save testcase")?;
        save_reason(&crash_dir, reason).with_context(|| "failed to save reason")?;

        info!("crash saved at '{}'", crash_dir);

        Ok(())
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

pub fn parse_trace(dir: &LocalPath) -> anyhow::Result<Trace> {
    let trace = fs::read_to_string(dir.join(TRACE_FILENAME))?;
    anyhow::Ok(Trace::try_parse(trace).with_context(|| "failed to parse trace")?)
}

pub fn setup_dir(cmdi: &dyn CommandInterface, path: &RemotePath) -> anyhow::Result<()> {
    cmdi.remove_dir_all(path).unwrap_or(());
    cmdi.create_dir_all(path)
}
