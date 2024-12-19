use std::{
    cell::RefCell,
    fs, io,
    path::Path,
    rc::Rc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use log::{debug, error, info};
use rand::{rngs::StdRng, SeedableRng};

use crate::{
    abstract_fs::{
        compile::{TEST_EXE_FILENAME, TEST_SOURCE_FILENAME},
        encode::encode_c,
        types::Workload,
    },
    config::Config,
    greybox::objective::{console::ConsoleObjective, trace::TraceObjective},
    mount::{btrfs::Btrfs, ext4::Ext4},
    temp_dir::setup_temp_dir,
};

use super::{feedback::kcov::KCovFeedback, harness::Harness, mutator::Mutator};

pub struct Fuzzer {
    corpus: Vec<Workload>,
    next_seed: usize,

    fst_exec_dir: Box<Path>,
    snd_exec_dir: Box<Path>,
    test_dir: Box<Path>,
    crashes_path: Box<Path>,

    fst_kcov_feedback: KCovFeedback,
    snd_kcov_feedback: KCovFeedback,

    trace_objective: TraceObjective,
    console_objective: ConsoleObjective,

    fst_harness: Harness<Ext4>,
    snd_harness: Harness<Btrfs>,

    mutator: Mutator,

    stats: Stats,
}

struct Stats {
    executions: usize,
    crashes: usize,
    start: Instant,
}

impl Stats {
    fn new() -> Self {
        Stats {
            executions: 0,
            crashes: 0,
            start: Instant::now(),
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
        let fst_trace_path = fst_exec_dir.join("trace.csv");
        let fst_kcov_path = fst_exec_dir.join("kcov.dat");
        let snd_trace_path = snd_exec_dir.join("trace.csv");
        let snd_kcov_path = snd_exec_dir.join("kcov.dat");

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

        let fst_harness = Harness::new(
            Ext4::new(),
            Path::new("/mnt")
                .join("ext4")
                .join("fstest")
                .into_boxed_path(),
            fst_exec_dir.clone().into_boxed_path(),
            fst_stdout,
            fst_stderr,
        );
        let snd_harness = Harness::new(
            Btrfs::new(),
            Path::new("/mnt")
                .join("btrfs")
                .join("fstest")
                .into_boxed_path(),
            snd_exec_dir.clone().into_boxed_path(),
            snd_stdout,
            snd_stderr,
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
            test_dir: test_dir.into_boxed_path(),
            crashes_path: crashes_path.to_path_buf().into_boxed_path(),

            fst_kcov_feedback,
            snd_kcov_feedback,

            trace_objective,
            console_objective,

            fst_harness,
            snd_harness,

            mutator,

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
        }
    }

    fn fuzz_one(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("picking input");
        let input = self.pick_input();

        debug!("mutating input");
        let input = self.mutator.mutate(input);

        debug!("compiling test at '{}'", self.test_dir.display());
        let input_path = input.compile(&self.test_dir)?;

        debug!(
            "setting up executable directories at '{}' and '{}'",
            self.fst_exec_dir.display(),
            self.snd_exec_dir.display()
        );
        setup_dir(self.fst_exec_dir.as_ref())?;
        setup_dir(self.snd_exec_dir.as_ref())?;

        debug!("running harness at '{}'", input_path.display());
        self.fst_harness.run(&input_path)?;
        self.snd_harness.run(&input_path)?;

        debug!("doing objectives");
        let console_is_interesting = self.console_objective.is_interesting()?;
        let trace_is_interesting = self.trace_objective.is_interesting()?;
        if console_is_interesting || trace_is_interesting {
            self.report_crash(input, &input_path)?;
            self.show_stats();
            return Ok(());
        }

        debug!("getting feedback");
        let fst_kcov_is_interesting = self.fst_kcov_feedback.is_interesting()?;
        let snd_kcov_is_interesting = self.snd_kcov_feedback.is_interesting()?;
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

    fn report_crash(&mut self, input: Workload, input_path: &Path) -> io::Result<()> {
        let name = input.generate_name();
        debug!("report crash '{}'", name);

        let crash_dir = self.crashes_path.join(name);
        debug!(
            "creating testcase crash directory at '{}'",
            crash_dir.display()
        );
        fs::create_dir(crash_dir.as_path())?;

        let source_path = crash_dir.join(TEST_SOURCE_FILENAME);
        debug!("saving source code at '{}'", source_path.display());
        fs::write(source_path, encode_c(input.clone()))?;

        let exe_path = crash_dir.join(TEST_EXE_FILENAME);
        debug!(
            "copying executable from '{}' to '{}'",
            input_path.display(),
            exe_path.display()
        );
        fs::copy(input_path, exe_path)?;

        let json_path = crash_dir.join("test").with_extension("json");
        debug!("saving workload as json at '{}'", json_path.display());
        let json = serde_json::to_string_pretty(&input)?;
        fs::write(json_path, json)?;

        self.stats.crashes += 1;
        Ok(())
    }

    fn add_to_corpus(&mut self, input: Workload) {
        debug!("adding new input to corpus");
        self.corpus.push(input);
    }

    fn show_stats(&self) {
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
