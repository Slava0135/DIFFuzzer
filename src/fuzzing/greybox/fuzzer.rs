use std::{
    cell::RefCell,
    fs::{self, read_to_string},
    io,
    path::Path,
    rc::Rc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Ok};
use log::{debug, error, info, warn};
use rand::{rngs::StdRng, SeedableRng};

use crate::fuzzing::greybox::feedback::kcov::KCOV_FILENAME;
use crate::{
    abstract_fs::{
        trace::{Trace, TRACE_FILENAME},
        workload::Workload,
    },
    fuzzing::abstract_fuzzer::objective::{
        console::ConsoleObjective,
        trace::TraceObjective,
    },
    config::Config,
    harness::{ConsolePipe, Harness},
    mount::mount::FileSystemMount,
    save::{save_output, save_testcase},
    temp_dir::setup_temp_dir,
};
use crate::fuzzing::abstract_fuzzer::fuzzer_data::FuzzData;
use crate::fuzzing::abstract_fuzzer::utils::{parse_trace, setup_dir};

use super::{feedback::kcov::KCovFeedback, mutator::Mutator};

pub struct Fuzzer {
    data: FuzzData,

    corpus: Vec<Workload>,
    next_seed: usize,

    fst_kcov_feedback: KCovFeedback,
    snd_kcov_feedback: KCovFeedback,

    mutator: Mutator,

    heartbeat_interval: u16,
}


impl Fuzzer {
    pub fn new(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
    ) -> Self {
        let fuzz_data = FuzzData::new(fst_mount, snd_mount);

        let fst_kcov_path = fuzz_data.fst_exec_dir.join(KCOV_FILENAME);
        let snd_kcov_path = fuzz_data.snd_exec_dir.join(KCOV_FILENAME);


        let fst_kcov_feedback = KCovFeedback::new(fst_kcov_path.clone().into_boxed_path());
        let snd_kcov_feedback = KCovFeedback::new(snd_kcov_path.clone().into_boxed_path());


        let mutator = Mutator::new(
            StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            ),
            config.operation_weights.clone(),
            config.mutation_weights.clone(),
            config.max_workload_length,
            config.greybox.max_mutations,
        );

        Self {
            data: fuzz_data,
            corpus: vec![Workload::new()],
            next_seed: 0,

            fst_kcov_feedback,
            snd_kcov_feedback,

            mutator,

            heartbeat_interval: config.greybox.heartbeat_interval,
        }
    }

    pub fn fuzz(&mut self) {
        info!("starting fuzzing loop");
        self.data.stats.start = Instant::now();
        loop {
            match self.fuzz_one() {
                Err(err) => {
                    error!("{:?}", err);
                    return;
                }
                _ => self.data.stats.executions += 1,
            }
            if Instant::now()
                .duration_since(self.data.stats.last_time_showed)
                .as_secs()
                > self.heartbeat_interval.into()
            {
                self.data.show_stats();
            }
        }
    }

    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        debug!("picking input");
        let input = self.pick_input();

        debug!("mutating input");
        let input = self.mutator.mutate(input);

        debug!("compiling test at '{}'", self.data.test_dir.display());
        let input_path = input
            .compile(&self.data.test_dir)
            .with_context(|| format!("failed to compile test"))?;

        debug!("running harness at '{}'", input_path.display());

        setup_dir(self.data.fst_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", input_path.display()))?;
        setup_dir(self.data.snd_exec_dir.as_ref())
            .with_context(|| format!("failed to setup dir at '{}'", input_path.display()))?;

        self.data.fst_harness
            .run(&input_path)
            .with_context(|| format!("failed to run first harness '{}'", self.data.fst_fs_name))?;
        self.data.snd_harness
            .run(&input_path)
            .with_context(|| format!("failed to run second harness '{}'", self.data.snd_fs_name))?;

        debug!("checking results");
        let fst_trace = parse_trace(&self.data.fst_trace_path)
            .with_context(|| format!("failed to parse first trace"))?;
        let snd_trace = parse_trace(&self.data.snd_trace_path)
            .with_context(|| format!("failed to parse second trace"))?;

        if fst_trace.has_errors() && snd_trace.has_errors() {
            warn!("both traces contain errors, potential bug in model");
            self.data.report_crash(input, &input_path, self.data.accidents_path.clone())
                .with_context(|| format!("failed to report accident"))?;
            return Ok(());
        }

        debug!("doing objectives");
        let console_is_interesting = self
            .data
            .console_objective
            .is_interesting()
            .with_context(|| format!("failed to do console objective"))?;
        let trace_is_interesting = self
            .data
            .trace_objective
            .is_interesting(&fst_trace, &snd_trace)
            .with_context(|| format!("failed to do trace objective"))?;
        if console_is_interesting || trace_is_interesting {
            self.data.report_crash(input, &input_path, self.data.crashes_path.clone())
                .with_context(|| format!("failed to report crash"))?;
            self.data.stats.crashes += 1;
            self.show_stats_corpus();
            self.data.show_stats();
            return Ok(());
        }

        debug!("getting feedback");
        let fst_kcov_is_interesting =
            self.fst_kcov_feedback.is_interesting().with_context(|| {
                format!(
                    "failed to get first kcov feedback for '{}'",
                    self.data.fst_fs_name
                )
            })?;
        let snd_kcov_is_interesting =
            self.snd_kcov_feedback.is_interesting().with_context(|| {
                format!(
                    "failed to get second kcov feedback for '{}'",
                    self.data.snd_fs_name
                )
            })?;
        if fst_kcov_is_interesting || snd_kcov_is_interesting {
            self.add_to_corpus(input);
            self.data.show_stats();
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


    fn add_to_corpus(&mut self, input: Workload) {
        debug!("adding new input to corpus");
        self.corpus.push(input);
    }

    fn show_stats_corpus(&self) {
        info!("corpus: {}, ", self.corpus.len());
    }
}