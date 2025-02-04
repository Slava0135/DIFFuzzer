use std::fs;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Ok};
use log::{debug, info};
use rand::{rngs::StdRng, SeedableRng};

use crate::fuzzing::common::{parse_trace, Fuzzer, Runner};
use crate::fuzzing::greybox::feedback::kcov::KCOV_FILENAME;
use crate::save::{save_output, save_testcase};
use crate::{abstract_fs::workload::Workload, config::Config, mount::mount::FileSystemMount};

use super::{feedback::kcov::KCovFeedback, mutator::Mutator};

pub struct GreyBoxFuzzer {
    runner: Runner,

    corpus: Vec<Workload>,
    next_seed: usize,

    fst_kcov_feedback: KCovFeedback,
    snd_kcov_feedback: KCovFeedback,

    mutator: Mutator,

    corpus_path: Option<Box<Path>>,
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

        let corpus_path = if config.greybox.save_corpus {
            let path = Path::new("./corpus");
            fs::create_dir(path).unwrap_or(());
            Some(path.to_path_buf().into_boxed_path())
        } else {
            None
        };

        let runner = Runner::new(fst_mount, snd_mount, config);

        let fst_kcov_path = runner.fst_exec_dir.join(KCOV_FILENAME);
        let snd_kcov_path = runner.snd_exec_dir.join(KCOV_FILENAME);

        let fst_kcov_feedback = KCovFeedback::new(fst_kcov_path.clone().into_boxed_path());
        let snd_kcov_feedback = KCovFeedback::new(snd_kcov_path.clone().into_boxed_path());

        Self {
            runner,
            corpus: vec![Workload::new()],
            next_seed: 0,

            fst_kcov_feedback,
            snd_kcov_feedback,

            mutator,

            corpus_path,
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

    fn save_input(&mut self, input: Workload, input_path: &Path) -> anyhow::Result<()> {
        let name = input.generate_name();
        debug!("save corpus input '{}'", name);

        let corpus_dir = self.corpus_path.clone().unwrap().join(name);
        if fs::exists(corpus_dir.as_path()).with_context(|| {
            format!(
                "failed to determine existence of corpus directory at '{}'",
                corpus_dir.display()
            )
        })? {
            return anyhow::Ok(());
        }
        fs::create_dir(corpus_dir.as_path()).with_context(|| {
            format!(
                "failed to create corpus directory at '{}'",
                corpus_dir.display()
            )
        })?;

        save_testcase(&corpus_dir, input_path, &input)?;
        save_output(
            &corpus_dir,
            &self.runner.fst_trace_path,
            &self.runner.fst_fs_name,
            self.runner.fst_stdout.borrow().clone(),
            self.runner.fst_stderr.borrow().clone(),
        )
        .with_context(|| format!("failed to save output for first harness"))?;
        save_output(
            &corpus_dir,
            &self.runner.snd_trace_path,
            &self.runner.snd_fs_name,
            self.runner.snd_stdout.borrow().clone(),
            self.runner.snd_stderr.borrow().clone(),
        )
        .with_context(|| format!("failed to save output for first harness"))?;
        Ok(())
    }
}

impl Fuzzer for GreyBoxFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        debug!("picking input");
        let input = self.pick_input();

        debug!("mutating input");
        let input = self.mutator.mutate(input);

        let input_path = self.runner().compile_test(&input)?;

        self.runner().run_harness(&input_path)?;

        let fst_trace = parse_trace(&self.runner().fst_trace_path)
            .with_context(|| format!("failed to parse first trace"))?;
        let snd_trace = parse_trace(&self.runner().snd_trace_path)
            .with_context(|| format!("failed to parse second trace"))?;

        if self.detect_errors(&input, &input_path, &fst_trace, &snd_trace)? {
            return Ok(());
        }

        if self.do_objective(&input, &input_path, &fst_trace, &snd_trace)? {
            return Ok(());
        }

        debug!("getting feedback");
        let fst_kcov_is_interesting =
            self.fst_kcov_feedback.is_interesting().with_context(|| {
                format!(
                    "failed to get first kcov feedback for '{}'",
                    self.runner.fst_fs_name
                )
            })?;
        let snd_kcov_is_interesting =
            self.snd_kcov_feedback.is_interesting().with_context(|| {
                format!(
                    "failed to get second kcov feedback for '{}'",
                    self.runner.snd_fs_name
                )
            })?;
        if fst_kcov_is_interesting || snd_kcov_is_interesting {
            self.add_to_corpus(input.clone());
            self.show_stats();
            if self.corpus_path.is_some() {
                self.save_input(input, &input_path)
                    .with_context(|| format!("failed to save input"))?;
            }
            return Ok(());
        }

        Ok(())
    }

    fn show_stats(&mut self) {
        self.runner.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.runner.stats.start);
        let secs = since_start.as_secs();
        info!(
            "corpus: {}, crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.corpus.len(),
            self.runner.stats.crashes,
            self.runner.stats.executions,
            (self.runner.stats.executions as f64) / (secs as f64),
            secs / (60 * 60),
            (secs / (60)) % 60,
            secs % 60,
        );
    }

    fn runner(&mut self) -> &mut Runner {
        &mut self.runner
    }
}
