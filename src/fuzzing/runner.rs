use crate::abstract_fs::trace::{Trace, TRACE_FILENAME};

use crate::abstract_fs::workload::Workload;
use crate::command::{CommandInterface, LocalCommandInterface};
use crate::config::Config;
use crate::hasher::hasher::FileDiff;
use crate::mount::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};
use crate::save::{save_diff, save_output, save_testcase};
use anyhow::{Context, Ok};
use log::{debug, info};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use super::harness::{ConsolePipe, Harness};
use super::objective::hash::HashObjective;
use super::objective::trace::TraceObjective;

pub struct Runner {
    pub config: Config,

    pub cmdi: Box<dyn CommandInterface>,

    pub test_dir: RemotePath,
    pub fst_exec_dir: RemotePath,
    pub snd_exec_dir: RemotePath,
    pub fst_trace_path: RemotePath,
    pub snd_trace_path: RemotePath,

    pub fst_stdout: ConsolePipe,
    pub snd_stdout: ConsolePipe,
    pub fst_stderr: ConsolePipe,
    pub snd_stderr: ConsolePipe,

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
    pub fn new(
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        config: Config,
    ) -> Self {
        info!("new fuzzer");

        let cmdi = Box::new(LocalCommandInterface::new());

        let temp_dir = cmdi
            .setup_remote_dir()
            .with_context(|| "failed to setup temp dir")
            .unwrap();

        info!("setting up fuzzing components");
        let test_dir = temp_dir.clone();
        let fst_exec_dir = temp_dir.join("fst_exec");
        let snd_exec_dir = temp_dir.join("snd_exec");

        let fst_trace_path = fst_exec_dir.join(TRACE_FILENAME);
        let snd_trace_path = snd_exec_dir.join(TRACE_FILENAME);

        let crashes_path = LocalPath::new(Path::new("./crashes"));
        fs::create_dir(&crashes_path).unwrap_or(());

        let accidents_path = LocalPath::new(Path::new("./accidents"));
        fs::create_dir(&accidents_path).unwrap_or(());

        let fst_stdout = Rc::new(RefCell::new("".to_owned()));
        let fst_stderr = Rc::new(RefCell::new("".to_owned()));
        let snd_stdout = Rc::new(RefCell::new("".to_owned()));
        let snd_stderr = Rc::new(RefCell::new("".to_owned()));

        let fst_fs_name = fst_mount.to_string();
        let snd_fs_name = snd_mount.to_string();

        let fst_fs_dir = RemotePath::new(Path::new("/mnt"))
            .join(fst_fs_name.to_lowercase())
            .join(&config.fs_name);
        let snd_fs_dir = RemotePath::new(Path::new("/mnt"))
            .join(snd_fs_name.to_lowercase())
            .join(&config.fs_name);

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
            fst_exec_dir.clone(),
            fst_stdout.clone(),
            fst_stderr.clone(),
        );
        let snd_harness = Harness::new(
            snd_mount,
            snd_fs_dir.clone(),
            snd_exec_dir.clone(),
            snd_stdout.clone(),
            snd_stderr.clone(),
        );

        Self {
            config,

            cmdi,

            fst_exec_dir: fst_exec_dir,
            snd_exec_dir: snd_exec_dir,
            fst_trace_path: fst_trace_path,
            snd_trace_path: snd_trace_path,

            fst_stdout,
            snd_stdout,
            fst_stderr,
            snd_stderr,

            test_dir: test_dir,
            crashes_path: crashes_path,
            accidents_path: accidents_path,

            hash_objective,
            trace_objective,

            fst_fs_name,
            snd_fs_name,
            fst_harness,
            snd_harness,

            stats: Stats::new(),
        }
    }

    pub fn compile_test(&mut self, input: &Workload) -> anyhow::Result<RemotePath> {
        debug!("compiling test at '{}'", self.test_dir);
        let binary_path = input
            .compile(self.cmdi.as_ref(), &self.test_dir)
            .with_context(|| format!("failed to compile test"))?;
        Ok(binary_path)
    }

    pub fn run_harness(&mut self, binary_path: &RemotePath) -> anyhow::Result<()> {
        debug!("running harness at '{}'", binary_path);

        setup_dir(self.cmdi.as_ref(), &self.fst_exec_dir).with_context(|| {
            format!(
                "failed to setup remote exec dir at '{}'",
                &self.fst_exec_dir
            )
        })?;
        setup_dir(self.cmdi.as_ref(), &self.snd_exec_dir).with_context(|| {
            format!(
                "failed to setup remote exec dir at '{}'",
                &self.snd_exec_dir
            )
        })?;

        self.fst_harness
            .run(
                self.cmdi.as_ref(),
                &binary_path,
                false,
                Some(&mut self.hash_objective.fst_fs),
            )
            .with_context(|| format!("failed to run first harness '{}'", self.fst_fs_name))?;
        self.snd_harness
            .run(
                self.cmdi.as_ref(),
                &binary_path,
                false,
                Some(&mut self.hash_objective.snd_fs),
            )
            .with_context(|| format!("failed to run second harness '{}'", self.snd_fs_name))?;
        Ok(())
    }

    pub fn report_crash(
        &mut self,
        input: &Workload,
        binary_path: &RemotePath,
        crash_dir: LocalPath,
        hash_diff: Vec<FileDiff>,
    ) -> anyhow::Result<()> {
        let name = input.generate_name();
        debug!("report crash '{}'", name);

        let crash_dir = crash_dir.join(name);
        if fs::exists(&crash_dir).with_context(|| {
            format!(
                "failed to determine existence of crash directory at '{}'",
                crash_dir
            )
        })? {
            return anyhow::Ok(());
        }
        fs::create_dir(&crash_dir)
            .with_context(|| format!("failed to create crash directory at '{}'", crash_dir))?;

        save_testcase(self.cmdi.as_ref(), &crash_dir, binary_path, &input)?;
        save_output(
            self.cmdi.as_ref(),
            &crash_dir,
            &self.fst_trace_path,
            &self.fst_fs_name,
            self.fst_stdout.borrow().clone(),
            self.fst_stderr.borrow().clone(),
        )
        .with_context(|| format!("failed to save output for first harness"))?;
        save_output(
            self.cmdi.as_ref(),
            &crash_dir,
            &self.snd_trace_path,
            &self.snd_fs_name,
            self.snd_stdout.borrow().clone(),
            self.snd_stderr.borrow().clone(),
        )
        .with_context(|| format!("failed to save output for first harness"))?;

        save_diff(&crash_dir, hash_diff)
            .with_context(|| format!("failed to save hash differences"))?;
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

pub fn parse_trace(cmdi: &dyn CommandInterface, path: &RemotePath) -> anyhow::Result<Trace> {
    let trace = cmdi.read_to_string(path)?;
    anyhow::Ok(Trace::try_parse(trace).with_context(|| format!("failed to parse trace"))?)
}

pub fn setup_dir(cmdi: &dyn CommandInterface, path: &RemotePath) -> anyhow::Result<()> {
    cmdi.remove_dir_all(path).unwrap_or(());
    cmdi.create_dir_all(path)
}
