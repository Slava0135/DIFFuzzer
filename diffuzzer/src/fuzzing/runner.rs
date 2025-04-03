/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::abstract_fs::trace::{TRACE_FILENAME, Trace};

use crate::abstract_fs::workload::Workload;
use crate::command::CommandInterface;
use crate::config::Config;
use crate::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};
use crate::reason::Reason;
use crate::save::{save_completed, save_reason, save_testcase};
use crate::supervisor::Supervisor;
use anyhow::{Context, Ok};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use super::broker::BrokerHandle;
use super::harness::Harness;
use super::objective::dash::DashObjective;
use super::objective::trace::TraceObjective;
use super::observer::ObserverList;
use super::observer::dash::DashObserver;
use super::outcome::{Completed, DiffCompleted, DiffOutcome, Outcome};

pub struct Runner {
    pub config: Config,

    pub keep_fs: bool,

    pub cmdi: Box<dyn CommandInterface>,
    pub supervisor: Box<dyn Supervisor>,

    /// Directory with executor and test source.
    pub test_dir: RemotePath,

    pub crashes_path: LocalPath,
    pub accidents_path: LocalPath,

    pub trace_objective: TraceObjective,
    pub dash_objective: DashObjective,

    pub fst_fs_name: String,
    pub snd_fs_name: String,

    pub fst_harness: Harness,
    pub snd_harness: Harness,

    pub executions: u64,
    pub crashes: u64,

    pub broker: BrokerHandle,
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
        local_tmp_dir: LocalPath,
        broker: BrokerHandle,
        mut observers: (ObserverList, ObserverList),
    ) -> anyhow::Result<Self> {
        let remote_tmp_dir = cmdi
            .setup_remote_dir()
            .with_context(|| "failed to setup remote temporary dir")?;

        let test_dir = remote_tmp_dir.clone();
        let exec_dir = remote_tmp_dir.join("exec");

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

        let fst_dash_observer = Rc::new(RefCell::new(
            DashObserver::create(
                &config,
                cmdi.as_ref(),
                fst_fs_dir.clone(),
                fst_mount.get_internal_dirs(),
            )
            .with_context(|| "failed to create first Dash observer")?,
        ));
        let snd_dash_observer = Rc::new(RefCell::new(
            DashObserver::create(
                &config,
                cmdi.as_ref(),
                snd_fs_dir.clone(),
                snd_mount.get_internal_dirs(),
            )
            .with_context(|| "failed to create first Dash observer")?,
        ));
        observers.0.push(fst_dash_observer.clone());
        observers.1.push(snd_dash_observer.clone());

        let dash_objective = DashObjective::new(&config, fst_dash_observer, snd_dash_observer);
        let trace_objective = TraceObjective::new();

        let fst_harness = Harness::new(
            fst_mount,
            fst_fs_dir.clone(),
            exec_dir.clone(),
            local_tmp_dir.join("outcome-1"),
            config.timeout,
            observers.0,
        );
        let snd_harness = Harness::new(
            snd_mount,
            snd_fs_dir.clone(),
            exec_dir.clone(),
            local_tmp_dir.join("outcome-2"),
            config.timeout,
            observers.1,
        );

        let runner = Self {
            config,
            keep_fs,

            cmdi,
            supervisor,

            test_dir,
            crashes_path,
            accidents_path,

            dash_objective,
            trace_objective,

            fst_fs_name,
            snd_fs_name,
            fst_harness,
            snd_harness,

            executions: 0,
            crashes: 0,

            broker,
        };

        runner
            .supervisor
            .save_snapshot()
            .with_context(|| "failed to save snapshot")?;

        Ok(runner)
    }

    pub fn compile_test(&mut self, input: &Workload) -> anyhow::Result<RemotePath> {
        let binary_path = input
            .compile(self.cmdi.as_ref(), &self.test_dir)
            .with_context(|| "failed to compile test")?;
        Ok(binary_path)
    }

    pub fn run_harness(&mut self, binary_path: &RemotePath) -> anyhow::Result<DiffOutcome> {
        let fst_outcome = self
            .fst_harness
            .run(
                self.cmdi.as_ref(),
                binary_path,
                self.keep_fs,
                self.supervisor.as_mut(),
            )
            .with_context(|| format!("failed to run first harness '{}'", self.fst_fs_name))?;
        let fst_outcome = match fst_outcome {
            Outcome::Panicked => {
                self.supervisor
                    .load_snapshot()
                    .with_context(|| "failed to load snapshot")?;
                return Ok(DiffOutcome::FirstPanicked {
                    fs_name: self.fst_fs_name.clone(),
                });
            }
            Outcome::TimedOut => {
                return Ok(DiffOutcome::FirstTimedOut {
                    fs_name: self.fst_fs_name.clone(),
                    timeout: self.config.timeout,
                });
            }
            Outcome::Completed(completed) => completed,
        };

        let snd_outcome = self
            .snd_harness
            .run(
                self.cmdi.as_ref(),
                binary_path,
                self.keep_fs,
                self.supervisor.as_mut(),
            )
            .with_context(|| format!("failed to run second harness '{}'", self.snd_fs_name))?;

        let snd_outcome = match snd_outcome {
            Outcome::Panicked => {
                self.supervisor
                    .load_snapshot()
                    .with_context(|| "failed to load snapshot")?;
                return Ok(DiffOutcome::SecondPanicked {
                    fs_name: self.snd_fs_name.clone(),
                });
            }
            Outcome::TimedOut => {
                return Ok(DiffOutcome::SecondTimedOut {
                    fs_name: self.snd_fs_name.clone(),
                    timeout: self.config.timeout,
                });
            }
            Outcome::Completed(completed) => completed,
        };

        Ok(DiffOutcome::DiffCompleted(
            self.diff(fst_outcome, snd_outcome)?,
        ))
    }

    pub fn report_diff(
        &mut self,
        input: &Workload,
        dir_name: String,
        binary_path: &RemotePath,
        crash_dir: LocalPath,
        diff: &DiffCompleted,
        reason: Reason,
    ) -> anyhow::Result<()> {
        let crash_dir = crash_dir.join(dir_name);
        fs::create_dir_all(&crash_dir)
            .with_context(|| format!("failed to create crash directory at '{}'", crash_dir))?;

        save_testcase(self.cmdi.as_ref(), &crash_dir, Some(binary_path), input)
            .with_context(|| "failed to save testcase")?;
        save_completed(&crash_dir, &self.fst_fs_name, &diff.fst_outcome)
            .with_context(|| "failed to save first outcome")?;
        save_completed(&crash_dir, &self.snd_fs_name, &diff.snd_outcome)
            .with_context(|| "failed to save second outcome")?;

        save_reason(&crash_dir, reason).with_context(|| "failed to save reason")?;

        self.broker.info(format!("diff saved at '{}'", crash_dir))?;

        Ok(())
    }

    pub fn report_crash(
        &mut self,
        input: &Workload,
        dir_name: String,
        crash_dir: LocalPath,
        reason: Reason,
    ) -> anyhow::Result<()> {
        let crash_dir = crash_dir.join(dir_name);
        fs::create_dir_all(&crash_dir)
            .with_context(|| format!("failed to create crash directory at '{}'", crash_dir))?;

        save_testcase(self.cmdi.as_ref(), &crash_dir, None, input)
            .with_context(|| "failed to save testcase")?;
        save_reason(&crash_dir, reason).with_context(|| "failed to save reason")?;

        self.broker
            .info(format!("crash saved at '{}'", crash_dir))?;

        Ok(())
    }

    fn diff(
        &mut self,
        fst_outcome: Completed,
        snd_outcome: Completed,
    ) -> anyhow::Result<DiffCompleted> {
        let fst_trace =
            parse_trace(&fst_outcome.dir).with_context(|| "failed to parse first trace")?;
        let snd_trace =
            parse_trace(&snd_outcome.dir).with_context(|| "failed to parse second trace")?;

        let dash_interesting = self
            .dash_objective
            .is_interesting()
            .with_context(|| "failed to do dash objective")?;

        let dash_diff = if dash_interesting {
            self.dash_objective.diff()
        } else {
            vec![]
        };

        let trace_diff = self.trace_objective.diff(&fst_trace, &snd_trace);

        Ok(DiffCompleted {
            dash_diff,
            trace_diff,
            fst_outcome,
            snd_outcome,
            fst_trace,
            snd_trace,
        })
    }
}

pub fn parse_trace(dir: &LocalPath) -> anyhow::Result<Trace> {
    let trace = fs::read_to_string(dir.join(TRACE_FILENAME))?;
    anyhow::Ok(Trace::try_parse(trace).with_context(|| "failed to parse trace")?)
}
