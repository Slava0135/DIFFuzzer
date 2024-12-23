use std::{
    cell::RefCell,
    fs, io,
    path::Path,
    rc::Rc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use log::{debug, error, info};
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    abstract_fs::{
        compile::{TEST_EXE_FILENAME, TEST_SOURCE_FILENAME},
        types::{ConsolePipe, Workload},
    },
    config::Config,
    greybox::objective::{console::ConsoleObjective, trace::TraceObjective},
    mount::{btrfs::Btrfs, ext4::Ext4},
    temp_dir::setup_temp_dir,
};

use super::{feedback::kcov::KCovFeedback, harness::Harness, mutator::Mutator};

const TRACE_FILENAME: &str = "trace.csv";
const KCOV_FILENAME: &str = "kcov.dat";

pub struct Fuzzer {
    corpus: Vec<Workload>,
    next_seed: usize,

    fst_exec_dir: Box<Path>,
    snd_exec_dir: Box<Path>,
    fst_trace_path: Box<Path>,
    snd_trace_path: Box<Path>,

    fst_stdout: ConsolePipe,
    snd_stdout: ConsolePipe,
    fst_stderr: ConsolePipe,
    snd_stderr: ConsolePipe,

    test_dir: Box<Path>,
    crashes_path: Box<Path>,

    fst_kcov_feedback: KCovFeedback,
    snd_kcov_feedback: KCovFeedback,

    trace_objective: TraceObjective,
    console_objective: ConsoleObjective,

    fst_fs_name: String,
    snd_fs_name: String,
    fst_harness: Harness<Ext4>,
    snd_harness: Harness<Btrfs>,

    mutator: Mutator,

    heartbeat_interval: u16,

    stats: Stats,
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

impl Fuzzer {
    pub fn new(config: Config) -> Self {
        info!("new greybox fuzzer");

        info!("setting up temporary directory");
        let temp_dir = setup_temp_dir();

        info!("setting up fuzzing components");
        let test_dir = temp_dir.clone();
        let fst_exec_dir = temp_dir.join("fst_exec");
        let snd_exec_dir = temp_dir.join("snd_exec");
        let fst_trace_path = fst_exec_dir.join(TRACE_FILENAME);
        let fst_kcov_path = fst_exec_dir.join(KCOV_FILENAME);
        let snd_trace_path = snd_exec_dir.join(TRACE_FILENAME);
        let snd_kcov_path = snd_exec_dir.join(KCOV_FILENAME);

        let crashes_path = Path::new("./crashes");
        fs::create_dir(crashes_path).unwrap_or(());

        let fst_stdout = Rc::new(RefCell::new("".to_owned()));
        let fst_stderr = Rc::new(RefCell::new("".to_owned()));
        let snd_stdout = Rc::new(RefCell::new("".to_owned()));
        let snd_stderr = Rc::new(RefCell::new("".to_owned()));

        let fst_kcov_feedback = KCovFeedback::new(fst_kcov_path.clone().into_boxed_path());
        let snd_kcov_feedback = KCovFeedback::new(snd_kcov_path.clone().into_boxed_path());

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

        let fst_mount = Ext4::new();
        let fst_fs_name = fst_mount.to_string();
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
        let snd_mount = Btrfs::new();
        let snd_fs_name = snd_mount.to_string();
        let snd_harness = Harness::new(
            Btrfs::new(),
            Path::new("/mnt")
                .join(snd_fs_name.to_lowercase())
                .join("fstest")
                .into_boxed_path(),
            snd_exec_dir.clone().into_boxed_path(),
            snd_stdout.clone(),
            snd_stderr.clone(),
        );

        let mutator = Mutator::new(
            StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
            config.operation_weights.clone(),
            config.mutation_weights.clone(),
            config.greybox.max_workload_length,
            config.greybox.max_mutations,
        );

        Self {
            corpus: vec![Workload::new()],
            next_seed: 0,

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

            fst_kcov_feedback,
            snd_kcov_feedback,

            trace_objective,
            console_objective,

            fst_fs_name,
            snd_fs_name,

            fst_harness,
            snd_harness,

            mutator,

            heartbeat_interval: config.greybox.heartbeat_interval,

            stats: Stats::new(),
        }
    }

    pub fn fuzz(&mut self) {
        info!("starting fuzzing loop");
        self.stats.start = Instant::now();
        loop {
            match self.fuzz_one() {
                Err(err) => error!("{}", err),
                _ => self.stats.executions += 1,
            }
            if Instant::now()
                .duration_since(self.stats.last_time_showed)
                .as_secs()
                > self.heartbeat_interval.into()
            {
                self.show_stats();
            }
        }
    }

    fn fuzz_one(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("picking input");
        let input = self.pick_input();

        debug!("mutating input");
        let input = self.mutator.mutate(input);

        debug!("compiling test at '{}'", self.test_dir.display());
        let input_path = input
            .compile(&self.test_dir)
            .with_context(|| format!("failed to compile test"))?;

        debug!("running harness at '{}'", input_path.display());

        setup_dir(self.fst_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", input_path.display()))?;
        setup_dir(self.snd_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", input_path.display()))?;

        self.fst_harness
            .run(&input_path)
            .with_context(|| format!("failed to run first harness '{}'", self.fst_fs_name))?;
        self.snd_harness
            .run(&input_path)
            .with_context(|| format!("failed to run second harness '{}'", self.snd_fs_name))?;

        debug!("doing objectives");
        let console_is_interesting = self
            .console_objective
            .is_interesting()
            .with_context(|| format!("failed to do console objective"))?;
        let trace_is_interesting = self
            .trace_objective
            .is_interesting()
            .with_context(|| format!("failed to do trace objective"))?;
        if console_is_interesting || trace_is_interesting {
            self.report_crash(input, &input_path)
                .with_context(|| format!("failed to report crash"))?;
            self.show_stats();
            return Ok(());
        }

        debug!("getting feedback");
        let fst_kcov_is_interesting =
            self.fst_kcov_feedback.is_interesting().with_context(|| {
                format!(
                    "failed to get first kcov feedback for '{}'",
                    self.fst_fs_name
                )
            })?;
        let snd_kcov_is_interesting =
            self.snd_kcov_feedback.is_interesting().with_context(|| {
                format!(
                    "failed to get second kcov feedback for '{}'",
                    self.snd_fs_name
                )
            })?;
        if fst_kcov_is_interesting || snd_kcov_is_interesting {
            self.add_to_corpus(input);
            self.show_stats();
            return Ok(());
        }

        Ok(())
    }

    fn pick_input(&mut self) -> Workload {
        if self.next_seed >= self.corpus.len() {
            self.next_seed = 0
        }
        let workload = self.corpus.get(self.next_seed).unwrap().clone();
        self.next_seed += 1;
        workload
    }

    fn report_crash(&mut self, input: Workload, input_path: &Path) -> anyhow::Result<()> {
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

        let source_path = crash_dir.join(TEST_SOURCE_FILENAME);
        fs::write(&source_path, input.clone().encode_c()).with_context(|| {
            format!("failed to save source file to '{}'", source_path.display())
        })?;

        let exe_path = crash_dir.join(TEST_EXE_FILENAME);
        fs::copy(&input_path, &exe_path).with_context(|| {
            format!(
                "failed to copy executable from '{}' to '{}'",
                input_path.display(),
                exe_path.display()
            )
        })?;

        let json_path = crash_dir.join("test").with_extension("json");
        let json = serde_json::to_string_pretty(&input).with_context(|| {
            format!(
                "failed to copy workload as json at '{}'",
                json_path.display()
            )
        })?;
        fs::write(json_path, json)?;

        let fst_trace_path = crash_dir.join(format!("{}.{}", &self.fst_fs_name, TRACE_FILENAME));
        let snd_trace_path = crash_dir.join(format!("{}.{}", &self.snd_fs_name, TRACE_FILENAME));
        fs::copy(&self.fst_trace_path, &fst_trace_path).with_context(|| {
            format!(
                "failed to copy first trace from '{}' to '{}'",
                self.fst_trace_path.display(),
                fst_trace_path.display()
            )
        })?;
        fs::copy(&self.snd_trace_path, &snd_trace_path).with_context(|| {
            format!(
                "failed to copy second trace from '{}' to '{}'",
                self.snd_trace_path.display(),
                snd_trace_path.display()
            )
        })?;

        let fst_stdout_path = crash_dir.join(format!("{}.stdout.txt", &self.fst_fs_name));
        let snd_stdout_path = crash_dir.join(format!("{}.stdout.txt", &self.snd_fs_name));

        fs::write(&fst_stdout_path, self.fst_stdout.borrow().clone()).with_context(|| {
            format!(
                "failed to save first stdout at '{}'",
                fst_stdout_path.display()
            )
        })?;
        fs::write(&snd_stdout_path, self.snd_stdout.borrow().clone()).with_context(|| {
            format!(
                "failed to save second stdout at '{}'",
                fst_stdout_path.display()
            )
        })?;

        let fst_stderr_path = crash_dir.join(format!("{}.stderr.txt", &self.fst_fs_name));
        let snd_stderr_path = crash_dir.join(format!("{}.stderr.txt", &self.snd_fs_name));
        fs::write(&fst_stderr_path, self.fst_stderr.borrow().clone()).with_context(|| {
            format!(
                "failed to save first stderr at '{}'",
                fst_stderr_path.display()
            )
        })?;
        fs::write(&snd_stderr_path, self.snd_stderr.borrow().clone()).with_context(|| {
            format!(
                "failed to save second stdout at '{}'",
                snd_stderr_path.display()
            )
        })?;

        self.stats.crashes += 1;
        Ok(())
    }

    fn add_to_corpus(&mut self, input: Workload) {
        debug!("adding new input to corpus");
        self.corpus.push(input);
    }

    fn show_stats(&mut self) {
        self.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.stats.start);
        let secs = since_start.as_secs();
        info!(
            "corpus: {}, crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.corpus.len(),
            self.stats.crashes,
            self.stats.executions,
            (self.stats.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        );
    }
}

fn setup_dir(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path).unwrap_or(());
    fs::create_dir(path)
}
