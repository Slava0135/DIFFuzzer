use crate::abstract_fs::trace::{Trace, TRACE_FILENAME};

use crate::abstract_fs::workload::Workload;
use crate::command::{CommandInterface, LocalCommandInterface};
use crate::config::Config;
use crate::fuzzing::native::objective::trace::TraceObjective;
use crate::hasher::hasher::FileDiff;
use crate::mount::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};
use crate::save::{save_diff, save_output, save_testcase};
use crate::temp_dir::setup_temp_dir;
use anyhow::{Context, Ok};
use log::{debug, error, info, warn};
use std::cell::RefCell;
use std::fs::read_to_string;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;
use std::{fs, io};

use super::harness::{ConsolePipe, Harness};
use super::objective::hash::HashObjective;

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

pub trait Fuzzer {
    fn run(&mut self, test_count: Option<u64>) {
        info!("starting fuzzing loop");
        self.runner().stats.start = Instant::now();
        match test_count {
            None => loop {
                if self.runs() {
                    return;
                }
            },
            Some(count) => {
                for _ in 0..count {
                    if self.runs() {
                        return;
                    }
                }
            }
        }
    }

    fn runs(&mut self) -> bool {
        match self.fuzz_one() {
            Err(err) => {
                error!("{:?}", err);
                return true;
            }
            _ => self.runner().stats.executions += 1,
        }
        if Instant::now()
            .duration_since(self.runner().stats.last_time_showed)
            .as_secs()
            > self.runner().config.heartbeat_interval.into()
        {
            self.show_stats();
        }
        false
    }

    fn fuzz_one(&mut self) -> anyhow::Result<()>;

    fn do_objective(
        &mut self,
        input: &Workload,
        binary_path: &RemotePath,
        fst_trace: &Trace,
        snd_trace: &Trace,
    ) -> anyhow::Result<bool> {
        let runner = self.runner();
        debug!("doing objectives");
        let hash_diff_interesting = runner
            .hash_objective
            .is_interesting()
            .with_context(|| format!("failed to do hash objective"))?;
        let trace_is_interesting = runner
            .trace_objective
            .is_interesting(fst_trace, snd_trace)
            .with_context(|| format!("failed to do trace objective"))?;
        if trace_is_interesting || hash_diff_interesting {
            debug!(
                "Error detected by: trace?: {}, hash?: {}",
                trace_is_interesting, hash_diff_interesting
            );
            let mut diff: Vec<FileDiff> = vec![];
            if hash_diff_interesting {
                diff = runner.hash_objective.get_diff();
            }
            runner
                .report_crash(&input, binary_path, runner.crashes_path.clone(), diff)
                .with_context(|| format!("failed to report crash"))?;
            self.runner().stats.crashes += 1;
            self.show_stats();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn detect_errors(
        &mut self,
        input: &Workload,
        binary_path: &RemotePath,
        fst_trace: &Trace,
        snd_trace: &Trace,
    ) -> anyhow::Result<bool> {
        debug!("detecting errors");
        if fst_trace.has_errors() && snd_trace.has_errors() {
            warn!("both traces contain errors, potential bug in model");
            let accidents_path = self.runner().accidents_path.clone();
            self.runner()
                .report_crash(&input, binary_path, accidents_path, vec![])
                .with_context(|| format!("failed to report accident"))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn show_stats(&mut self);

    fn runner(&mut self) -> &mut Runner;
}

impl Runner {
    pub fn new(
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        config: Config,
    ) -> Self {
        info!("new fuzzer");

        let cmdi = Box::new(LocalCommandInterface::new());

        let temp_dir = setup_temp_dir(cmdi.as_ref())
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
            .compile(&self.test_dir)
            .with_context(|| format!("failed to compile test"))?;
        Ok(binary_path)
    }

    pub fn run_harness(&mut self, binary_path: &RemotePath) -> anyhow::Result<()> {
        debug!("running harness at '{}'", binary_path);

        setup_dir(&self.fst_exec_dir).with_context(|| {
            format!(
                "failed to setup remote exec dir at '{}'",
                &self.fst_exec_dir
            )
        })?;
        setup_dir(&self.snd_exec_dir).with_context(|| {
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

        save_testcase(&crash_dir, binary_path, &input)?;
        save_output(
            &crash_dir,
            &self.fst_trace_path,
            &self.fst_fs_name,
            self.fst_stdout.borrow().clone(),
            self.fst_stderr.borrow().clone(),
        )
        .with_context(|| format!("failed to save output for first harness"))?;
        save_output(
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

pub fn parse_trace(path: &RemotePath) -> anyhow::Result<Trace> {
    todo!("use cmdi");
    let trace = read_to_string(&path.base)
        .with_context(|| format!("failed to read trace at '{}'", path))?;
    anyhow::Ok(Trace::try_parse(trace).with_context(|| format!("failed to parse trace"))?)
}

pub fn setup_dir(path: &RemotePath) -> io::Result<()> {
    todo!("use cmdi");
    fs::remove_dir_all(path.base.as_ref()).unwrap_or(());
    fs::create_dir(path.base.as_ref())
}
