use crate::abstract_fs::trace::{Trace, TRACE_FILENAME};

use crate::abstract_fs::workload::Workload;
use crate::config::Config;
use crate::fuzzing::objective::trace::TraceObjective;
use crate::harness::{ConsolePipe, Harness};
use crate::hasher::hasher::FileDiff;
use crate::mount::mount::FileSystemMount;
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

use super::objective::hash::HashObjective;

pub struct Runner {
    pub config: Config,

    pub fst_exec_dir: Box<Path>,
    pub snd_exec_dir: Box<Path>,
    pub fst_trace_path: Box<Path>,
    pub snd_trace_path: Box<Path>,

    pub fst_stdout: ConsolePipe,
    pub snd_stdout: ConsolePipe,
    pub fst_stderr: ConsolePipe,
    pub snd_stderr: ConsolePipe,

    pub test_dir: Box<Path>,
    pub crashes_path: Box<Path>,
    pub accidents_path: Box<Path>,

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
        input_path: &Path,
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
                .report_crash(&input, input_path, runner.crashes_path.clone(), diff)
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
        input_path: &Path,
        fst_trace: &Trace,
        snd_trace: &Trace,
    ) -> anyhow::Result<bool> {
        debug!("detecting errors");
        if fst_trace.has_errors() && snd_trace.has_errors() {
            warn!("both traces contain errors, potential bug in model");
            let accidents_path = self.runner().accidents_path.clone();
            self.runner()
                .report_crash(&input, &input_path, accidents_path, vec![])
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

        let temp_dir = setup_temp_dir();

        info!("setting up fuzzing components");
        let test_dir = temp_dir.clone();
        let fst_exec_dir = temp_dir.join("fst_exec");
        let snd_exec_dir = temp_dir.join("snd_exec");

        let fst_trace_path = fst_exec_dir.join(TRACE_FILENAME);
        let snd_trace_path = snd_exec_dir.join(TRACE_FILENAME);

        let crashes_path = Path::new("./crashes");
        fs::create_dir(crashes_path).unwrap_or(());

        let accidents_path = Path::new("./accidents");
        fs::create_dir(accidents_path).unwrap_or(());

        let fst_stdout = Rc::new(RefCell::new("".to_owned()));
        let fst_stderr = Rc::new(RefCell::new("".to_owned()));
        let snd_stdout = Rc::new(RefCell::new("".to_owned()));
        let snd_stderr = Rc::new(RefCell::new("".to_owned()));

        let fst_fs_name = fst_mount.to_string();
        let snd_fs_name = snd_mount.to_string();

        let fst_fs_dir = Path::new("/mnt")
            .join(fst_fs_name.to_lowercase())
            .join(&config.fs_name)
            .into_boxed_path();
        let snd_fs_dir = Path::new("/mnt")
            .join(snd_fs_name.to_lowercase())
            .join(&config.fs_name)
            .into_boxed_path();

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
            fst_exec_dir.clone().into_boxed_path(),
            fst_stdout.clone(),
            fst_stderr.clone(),
        );
        let snd_harness = Harness::new(
            snd_mount,
            snd_fs_dir.clone(),
            snd_exec_dir.clone().into_boxed_path(),
            snd_stdout.clone(),
            snd_stderr.clone(),
        );

        Self {
            config,

            fst_exec_dir: fst_exec_dir.into_boxed_path(),
            snd_exec_dir: snd_exec_dir.into_boxed_path(),
            fst_trace_path: fst_trace_path.into_boxed_path(),
            snd_trace_path: snd_trace_path.into_boxed_path(),

            fst_stdout,
            snd_stdout,
            fst_stderr,
            snd_stderr,

            test_dir: test_dir.into_boxed_path(),
            crashes_path: crashes_path.to_path_buf().into_boxed_path(),
            accidents_path: accidents_path.to_path_buf().into_boxed_path(),

            hash_objective,
            trace_objective,

            fst_fs_name,
            snd_fs_name,
            fst_harness,
            snd_harness,

            stats: Stats::new(),
        }
    }

    pub fn compile_test(&mut self, input: &Workload) -> anyhow::Result<Box<Path>> {
        debug!("compiling test at '{}'", self.test_dir.display());
        let input_path = input
            .compile(&self.test_dir)
            .with_context(|| format!("failed to compile test"))?;
        Ok(input_path)
    }

    pub fn run_harness(&mut self, input_path: &Path) -> anyhow::Result<()> {
        debug!("running harness at '{}'", input_path.display());

        setup_dir(self.fst_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", input_path.display()))?;
        setup_dir(self.snd_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", input_path.display()))?;

        self.fst_harness
            .run(&input_path, false, Some(&mut self.hash_objective.fst_fs))
            .with_context(|| format!("failed to run first harness '{}'", self.fst_fs_name))?;
        self.snd_harness
            .run(&input_path, false, Some(&mut self.hash_objective.snd_fs))
            .with_context(|| format!("failed to run second harness '{}'", self.snd_fs_name))?;
        Ok(())
    }

    pub fn report_crash(
        &mut self,
        input: &Workload,
        input_path: &Path,
        crash_dir: Box<Path>,
        hash_diff: Vec<FileDiff>,
    ) -> anyhow::Result<()> {
        let name = input.generate_name();
        debug!("report crash '{}'", name);

        let crash_dir = crash_dir.join(name);
        if fs::exists(crash_dir.as_path()).with_context(|| {
            format!(
                "failed to determine existence of crash directory at '{}'",
                crash_dir.display()
            )
        })? {
            return anyhow::Ok(());
        }
        fs::create_dir(crash_dir.as_path()).with_context(|| {
            format!(
                "failed to create crash directory at '{}'",
                crash_dir.display()
            )
        })?;

        save_testcase(&crash_dir, input_path, &input)?;
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
        info!("crash saved at '{}'", crash_dir.display());

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

pub fn parse_trace(path: &Path) -> anyhow::Result<Trace> {
    let trace = read_to_string(&path)
        .with_context(|| format!("failed to read trace at '{}'", path.display()))?;
    anyhow::Ok(Trace::try_parse(trace).with_context(|| format!("failed to parse trace"))?)
}

pub fn setup_dir(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path).unwrap_or(());
    fs::create_dir(path)
}
