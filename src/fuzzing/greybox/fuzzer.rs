use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Ok};
use log::{debug, info, warn};
use rand::{rngs::StdRng, SeedableRng};

use crate::fuzzing::common::{parse_trace, setup_dir, FuzzData, Fuzzer};
use crate::fuzzing::greybox::feedback::kcov::KCOV_FILENAME;
use crate::hasher::hasher::{calc_dir_hash, get_diff, FileDiff};
use crate::{abstract_fs::workload::Workload, config::Config, mount::mount::FileSystemMount};

use super::{feedback::kcov::KCovFeedback, mutator::Mutator};

pub struct GreyBoxFuzzer {
    data: FuzzData,

    corpus: Vec<Workload>,
    next_seed: usize,

    fst_kcov_feedback: KCovFeedback,
    snd_kcov_feedback: KCovFeedback,

    mutator: Mutator,
}

impl GreyBoxFuzzer {
    pub fn new(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
    ) -> Self {
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

        let fuzz_data = FuzzData::new(fst_mount, snd_mount, config);

        let fst_kcov_path = fuzz_data.fst_exec_dir.join(KCOV_FILENAME);
        let snd_kcov_path = fuzz_data.snd_exec_dir.join(KCOV_FILENAME);

        let fst_kcov_feedback = KCovFeedback::new(fst_kcov_path.clone().into_boxed_path());
        let snd_kcov_feedback = KCovFeedback::new(snd_kcov_path.clone().into_boxed_path());

        Self {
            data: fuzz_data,
            corpus: vec![Workload::new()],
            next_seed: 0,

            fst_kcov_feedback,
            snd_kcov_feedback,

            mutator,
        }
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
}

impl Fuzzer for GreyBoxFuzzer {
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

        self.data
            .fst_harness
            .run(&input_path)
            .with_context(|| format!("failed to run first harness '{}'", self.data.fst_fs_name))?;
        self.data
            .snd_harness
            .run(&input_path)
            .with_context(|| format!("failed to run second harness '{}'", self.data.snd_fs_name))?;

        let fst_hash = calc_dir_hash(self.data.fst_exec_dir.as_ref(), &self.data.hasher_options);
        let snd_hash = calc_dir_hash(self.data.snd_exec_dir.as_ref(), &self.data.hasher_options);

        debug!("checking results");
        let fst_trace = parse_trace(&self.data.fst_trace_path)
            .with_context(|| format!("failed to parse first trace"))?;
        let snd_trace = parse_trace(&self.data.snd_trace_path)
            .with_context(|| format!("failed to parse second trace"))?;

        if fst_trace.has_errors() && snd_trace.has_errors() {
            warn!("both traces contain errors, potential bug in model");
            self.data
                .report_crash(input, &input_path, self.data.accidents_path.clone(), vec![])
                .with_context(|| format!("failed to report accident"))?;
            return Ok(());
        }

        let hash_diff_interesting = self.data.config.hashing_enabled && fst_hash != snd_hash;
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
        if console_is_interesting || trace_is_interesting || hash_diff_interesting {
            let mut diff: Vec<FileDiff> = vec![];
            if hash_diff_interesting {
                diff = get_diff(
                    &self.data.fst_exec_dir,
                    &self.data.snd_exec_dir,
                    &self.data.hasher_options,
                );
            }
            self.data
                .report_crash(input, &input_path, self.data.crashes_path.clone(), diff)
                .with_context(|| format!("failed to report crash"))?;
            self.data.stats.crashes += 1;
            self.show_stats();
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
            self.show_stats();
            return Ok(());
        }

        Ok(())
    }

    fn show_stats(&mut self) {
        self.data.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.data.stats.start);
        let secs = since_start.as_secs();
        info!(
            "corpus: {}, crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.corpus.len(),
            self.data.stats.crashes,
            self.data.stats.executions,
            (self.data.stats.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        );
    }

    fn data(&mut self) -> &mut FuzzData {
        &mut self.data
    }
}
