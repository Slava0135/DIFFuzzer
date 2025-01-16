use crate::abstract_fs::trace::TRACE_FILENAME;
use crate::config::Config;

use crate::abstract_fs::workload::Workload;
use crate::fuzzing::abstract_fuzzer::objective::console::ConsoleObjective;
use crate::fuzzing::abstract_fuzzer::objective::trace::TraceObjective;
use crate::harness::{ConsolePipe, Harness};
use crate::mount::btrfs::Btrfs;
use crate::mount::ext4::Ext4;
use crate::mount::mount::FileSystemMount;
use crate::save::{save_output, save_testcase};
use crate::temp_dir::setup_temp_dir;
use anyhow::Context;
use log::{debug, info};
use rand::prelude::StdRng;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct FuzzData {
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
    pub console_objective: ConsoleObjective,

    pub fst_fs_name: String,
    pub snd_fs_name: String,
    pub fst_harness: Harness,
    pub snd_harness: Harness,

    pub stats: Stats,
}

impl FuzzData {
    pub fn new(
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
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

        let trace_objective = TraceObjective::new();
        let console_objective = ConsoleObjective::new(fst_stdout.clone(), snd_stdout.clone());

        let fst_fs_name = fst_mount.to_string();
        let snd_fs_name = snd_mount.to_string();

        let fst_harness = Harness::new(
            fst_mount,
            Path::new("/mnt")
                .join(fst_fs_name.to_lowercase())
                .join("fstest")
                .into_boxed_path(),
            fst_exec_dir.clone().into_boxed_path(),
            fst_stdout.clone(),
            fst_stderr.clone(),
        );

        let snd_harness = Harness::new(
            snd_mount,
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
            accidents_path: accidents_path.to_path_buf().into_boxed_path(),

            trace_objective,
            console_objective,

            fst_fs_name,
            snd_fs_name,
            fst_harness,
            snd_harness,

            stats: Stats::new(),
        }
    }

    pub fn report_crash(
        &mut self,
        input: Workload,
        input_path: &Path,
        crash_dir: Box<Path>,
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

        info!("crash saved at '{}'", crash_dir.display());

        anyhow::Ok(())
    }

    pub fn show_stats(&mut self) {
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
