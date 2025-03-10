/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use log::{debug, info, warn};
use rand::{SeedableRng, rngs::StdRng};
use walkdir::WalkDir;

use crate::command::CommandInterface;
use crate::fuzzing::fuzzer::Fuzzer;
use crate::fuzzing::observer::ObserverList;
use crate::fuzzing::observer::lcov::LCovObserver;
use crate::fuzzing::outcome::{Completed, Outcome};
use crate::fuzzing::runner::Runner;
use crate::path::{LocalPath, RemotePath};
use crate::save::{TEST_FILE_NAME, save_completed, save_testcase};
use crate::supervisor::Supervisor;
use crate::{abstract_fs::workload::Workload, config::Config, mount::FileSystemMount};

use super::feedback::kcov::KCovCoverageFeedback;
use super::feedback::lcov::LCovCoverageFeedback;
use super::feedback::{CoverageFeedback, CoverageType, DummyCoverageFeedback, InputCoverage};
use super::mutator::Mutator;
use super::schedule::{FastPowerScheduler, QueueScheduler, Scheduler};
use super::seed::Seed;

pub struct GreyBoxFuzzer {
    runner: Runner,

    initial_corpus: Vec<Workload>,
    next_initial: usize,

    corpus: Vec<Seed>,
    scheduler: Box<dyn Scheduler>,

    fst_coverage_feedback: Box<dyn CoverageFeedback>,
    snd_coverage_feedback: Box<dyn CoverageFeedback>,

    mutator: Mutator,

    corpus_path: Option<LocalPath>,
}

impl GreyBoxFuzzer {
    pub fn create(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        corpus_path: Option<String>,
        cmdi: Box<dyn CommandInterface>,
        supervisor: Box<dyn Supervisor>,
    ) -> anyhow::Result<Self> {
        let mutator = Mutator::new(
            StdRng::seed_from_u64(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64),
            config.operation_weights.clone(),
            config.mutation_weights.clone(),
            config.max_workload_length,
            config.greybox.max_mutations,
        );

        let mut initial_corpus = Vec::new();
        if let Some(corpus_path) = corpus_path {
            for entry in WalkDir::new(&corpus_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name() == TEST_FILE_NAME)
            {
                match fs::read_to_string(entry.path()) {
                    Ok(data) => match serde_json::from_str(&data) {
                        Ok(workload) => initial_corpus.push(workload),
                        Err(err) => {
                            warn!(
                                "failed to parse seed at '{}':\n{}",
                                entry.path().display(),
                                err
                            )
                        }
                    },
                    Err(err) => warn!(
                        "failed to read seed data at '{}':\n{}",
                        entry.path().display(),
                        err
                    ),
                }
            }
            info!(
                "added {} seeds from '{}'",
                initial_corpus.len(),
                &corpus_path
            )
        };

        let corpus_path = if config.greybox.save_corpus {
            let path = Path::new("./corpus");
            fs::create_dir_all(path).with_context(|| "failed to create directory for corpus")?;
            Some(LocalPath::new(path))
        } else {
            None
        };

        let scheduler: Box<dyn Scheduler> = match config.greybox.scheduler {
            crate::config::Scheduler::Queue => Box::new(QueueScheduler::new()),
            crate::config::Scheduler::Fast => {
                Box::new(FastPowerScheduler::new(config.greybox.m_constant))
            }
        };

        let mut observers: (ObserverList, ObserverList) = (vec![], vec![]);

        let fst_coverage_feedback: Box<dyn CoverageFeedback> = match fst_mount.coverage_type() {
            CoverageType::None => Box::new(DummyCoverageFeedback::new()),
            CoverageType::LCov => {
                let source_dir = fst_mount
                    .source_dir()
                    .with_context(|| "Source directory is missing for first filesystem")?;
                let fst_lcov_observer = Rc::new(RefCell::new(LCovObserver::new(source_dir)));
                observers.0.push(fst_lcov_observer);
                Box::new(LCovCoverageFeedback::new())
            }
            CoverageType::KCov => Box::new(KCovCoverageFeedback::new()),
        };
        let snd_coverage_feedback: Box<dyn CoverageFeedback> = match snd_mount.coverage_type() {
            CoverageType::None => Box::new(DummyCoverageFeedback::new()),
            CoverageType::LCov => {
                let source_dir = snd_mount
                    .source_dir()
                    .with_context(|| "Source directory is missing for second filesystem")?;
                let snd_lcov_observer = Rc::new(RefCell::new(LCovObserver::new(source_dir)));
                observers.1.push(snd_lcov_observer);
                Box::new(LCovCoverageFeedback::new())
            }
            CoverageType::KCov => Box::new(KCovCoverageFeedback::new()),
        };

        let runner = Runner::create(
            fst_mount,
            snd_mount,
            crashes_path,
            config,
            false,
            cmdi,
            supervisor,
            observers,
        )
        .with_context(|| "failed to create runner")?;

        Ok(Self {
            runner,

            initial_corpus,
            next_initial: 0,

            corpus: vec![Seed::new(Workload::new(), HashSet::new(), HashSet::new())],
            scheduler,

            fst_coverage_feedback,
            snd_coverage_feedback,

            mutator,

            corpus_path,
        })
    }

    fn pick_input(&mut self) -> anyhow::Result<Workload> {
        if self.next_initial < self.initial_corpus.len() {
            let workload = self
                .initial_corpus
                .get(self.next_initial)
                .with_context(|| "failed to get seed from initial corpus")?
                .clone();
            self.next_initial += 1;
            Ok(workload)
        } else {
            let next = self.scheduler.choose(
                self.corpus.as_mut_slice(),
                self.fst_coverage_feedback.map(),
                self.snd_coverage_feedback.map(),
            )?;
            Ok(self.mutator.mutate(next))
        }
    }

    fn add_to_corpus(
        &mut self,
        input: Workload,
        fst_coverage: InputCoverage,
        snd_coverage: InputCoverage,
    ) {
        debug!("add new input to corpus");
        let seed = Seed::new(input, fst_coverage, snd_coverage);
        self.corpus.push(seed);
    }

    fn save_input(
        &mut self,
        input: Workload,
        binary_path: &RemotePath,
        fst_outcome: &Completed,
        snd_outcome: &Completed,
    ) -> anyhow::Result<()> {
        let name = input.generate_name();
        debug!("save corpus input '{}'", name);

        let corpus_dir = self.corpus_path.clone().unwrap().join(name);
        fs::create_dir_all(&corpus_dir)
            .with_context(|| format!("failed to create corpus directory at '{}'", corpus_dir))?;

        save_testcase(
            self.runner.cmdi.as_ref(),
            &corpus_dir,
            Some(binary_path),
            &input,
        )?;
        save_completed(&corpus_dir, &self.runner.fst_fs_name, fst_outcome)
            .with_context(|| "failed to save outcome for first harness")?;
        save_completed(&corpus_dir, &self.runner.snd_fs_name, snd_outcome)
            .with_context(|| "failed to save outcome for second harness")?;
        Ok(())
    }
}

impl Fuzzer for GreyBoxFuzzer {
    fn fuzz_one(&mut self) -> anyhow::Result<()> {
        debug!("pick input");
        let input = self.pick_input()?;

        let binary_path = self.runner().compile_test(&input)?;

        match self.runner().run_harness(&binary_path)? {
            (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                if self.detect_errors(&input, &binary_path, &fst_outcome, &snd_outcome)? {
                    return Ok(());
                }

                if self.do_objective(&input, &binary_path, &fst_outcome, &snd_outcome)? {
                    return Ok(());
                }

                debug!("get feedback");
                let fst_opinion = self
                    .fst_coverage_feedback
                    .opinion(&fst_outcome)
                    .with_context(|| "failed to get first coverage feedback")?;
                let snd_opinion = self
                    .snd_coverage_feedback
                    .opinion(&snd_outcome)
                    .with_context(|| "failed to get second coverage feedback")?;

                if fst_opinion.is_interesting() || snd_opinion.is_interesting() {
                    self.add_to_corpus(
                        input.clone(),
                        fst_opinion.coverage(),
                        snd_opinion.coverage(),
                    );
                    self.show_stats();
                    if self.corpus_path.is_some() {
                        self.save_input(input, &binary_path, &fst_outcome, &snd_outcome)
                            .with_context(|| "failed to save input")?;
                    }
                    return Ok(());
                }
            }
            (Outcome::Panicked, _) => {
                self.report_crash(
                    &input,
                    format!("Filesystem '{}' panicked", self.runner.fst_fs_name),
                )?;
            }
            (_, Outcome::Panicked) => {
                self.report_crash(
                    &input,
                    format!("Filesystem '{}' panicked", self.runner.snd_fs_name),
                )?;
            }
            (Outcome::TimedOut, _) => {
                self.report_crash(
                    &input,
                    format!(
                        "Filesystem '{}' timed out after {}s",
                        self.runner.fst_fs_name, self.runner.config.timeout
                    ),
                )?;
            }
            (_, Outcome::TimedOut) => {
                self.report_crash(
                    &input,
                    format!(
                        "Filesystem '{}' timed out after {}s",
                        self.runner.snd_fs_name, self.runner.config.timeout
                    ),
                )?;
            }
            (Outcome::Skipped, _) => {}
            (_, Outcome::Skipped) => {}
        };

        Ok(())
    }

    fn show_stats(&mut self) {
        self.runner.stats.last_time_showed = Instant::now();
        let since_start = Instant::now().duration_since(self.runner.stats.start);
        let secs = since_start.as_secs();
        info!(
            "corpus: {}, coverage: {} ({}) + {} ({}), crashes: {}, executions: {}, exec/s: {:.2}, time: {:02}h:{:02}m:{:02}s",
            self.corpus.len(),
            self.fst_coverage_feedback.map().len(),
            self.fst_coverage_feedback.coverage_type(),
            self.snd_coverage_feedback.map().len(),
            self.snd_coverage_feedback.coverage_type(),
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
