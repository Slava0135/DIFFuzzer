use crate::abstract_fs::trace::TRACE_FILENAME;
use crate::abstract_fs::types::{ConsolePipe, Workload};
use crate::config::Config;
use crate::greybox::feedback::kcov::{KCovFeedback, KCOV_FILENAME};
use crate::greybox::fuzzer::Fuzzer;
use crate::greybox::mutator::Mutator;

use crate::harness::Harness;
use crate::mount::btrfs::Btrfs;
use crate::mount::ext4::Ext4;
use crate::mount::mount::FileSystemMount;
use crate::temp_dir::setup_temp_dir;
use log::{debug, info};
use rand::prelude::StdRng;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use anyhow::Context;
use crate::abstract_fuzzer::objective::console::ConsoleObjective;
use crate::abstract_fuzzer::objective::trace::TraceObjective;
use crate::save::{save_output, save_testcase};

pub struct FuzzData<FS1: FileSystemMount, FS2: FileSystemMount> {
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

    pub trace_objective: TraceObjective,
    pub console_objective: ConsoleObjective,

    pub fst_fs_name: String,
    pub snd_fs_name: String,
    pub fst_harness: Harness<FS1>,
    pub snd_harness: Harness<FS2>,

    pub stats: Stats,
}

impl<FS1: FileSystemMount, FS2: FileSystemMount> FuzzData<FS1, FS2> {
    pub fn new(fst_fs: FS1, snd_fs: FS2) -> Self {
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

        let fst_stdout = Rc::new(RefCell::new("".to_owned()));
        let fst_stderr = Rc::new(RefCell::new("".to_owned()));
        let snd_stdout = Rc::new(RefCell::new("".to_owned()));
        let snd_stderr = Rc::new(RefCell::new("".to_owned()));

        let trace_objective = TraceObjective::new(
            fst_trace_path.clone().into_boxed_path(),
            snd_trace_path.clone().into_boxed_path(),
        );
        let console_objective = ConsoleObjective::new(
            fst_stdout.clone(),
            fst_stderr.clone(),
            snd_stdout.clone(),
            snd_stderr.clone(),
        );

        let fst_fs_name = fst_fs.to_string();
        let snd_fs_name = snd_fs.to_string();

        let fst_harness = Harness::new(
            fst_fs,
            Path::new("/mnt")
                .join(fst_fs_name.to_lowercase())
                .join("fstest")
                .into_boxed_path(),
            fst_exec_dir.clone().into_boxed_path(),
            fst_stdout.clone(),
            fst_stderr.clone(),
        );

        let snd_harness = Harness::new(
            snd_fs,
            Path::new("/mnt")
                .join(snd_fs_name.to_lowercase())
                .join("fstest")
                .into_boxed_path(),
            snd_exec_dir.clone().into_boxed_path(),
            snd_stdout.clone(),
            snd_stderr.clone(),
        );

        Self {
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

            trace_objective,
            console_objective,

            fst_fs_name,
            snd_fs_name,
            fst_harness,
            snd_harness,

            stats: Stats::new(),
        }
    }


    pub(crate) fn report_crash(&mut self, input: Workload, input_path: &Path) -> anyhow::Result<()> {
        let name = input.generate_name();
        debug!("report crash '{}'", name);

        let crash_dir = self.crashes_path.join(name);
        if fs::exists(crash_dir.as_path()).with_context(|| {
            format!(
                "failed to determine existence of crash directory at '{}'",
                crash_dir.display()
            )
        })? {
            return Ok(());
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

        self.stats.crashes += 1;
        Ok(())
    }

    pub(crate) fn show_stats(&mut self) {
        self.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.stats.start);
        let secs = since_start.as_secs();
        info!(
            "crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.stats.crashes,
            self.stats.executions,
            (self.stats.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        );
    }
}

struct Stats {
    executions: usize,
    crashes: usize,
    start: Instant,
    last_time_showed: Instant,
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
