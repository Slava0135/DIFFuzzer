/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::borrow::Borrow;
use std::fs::read_to_string;
use std::ops::Index;
use std::rc::Rc;
use std::thread::yield_now;

use anyhow::{Context, Ok};
use dash::FileDiff;
use log::{info, warn};

use crate::abstract_fs::trace::TraceRow;
use crate::fuzzing::objective::trace::TraceDiff;
use crate::fuzzing::outcome::Completed;
use crate::{
    abstract_fs::{mutator::remove, workload::Workload},
    command::CommandInterface,
    config::Config,
    fuzzing::{outcome::Outcome, runner::parse_trace},
    mount::FileSystemMount,
    path::LocalPath,
    supervisor::Supervisor,
};

use super::runner::Runner;

pub struct Reducer {
    runner: Runner,
}

#[derive(Clone)]
pub struct Bug {
    name: String,
    workload: Workload,
    dash_bug: Vec<FileDiff>,
    trace_bug: Vec<TraceDiff>,
    index: usize,
}

struct RunningResults {
    dash_interesting: bool,
    trace_interesting: bool,
    dash_diff: Vec<FileDiff>,
    trace_diff: Vec<TraceDiff>,
}

impl RunningResults {
    fn has_some_interesting(&self) -> bool {
        self.dash_interesting || self.trace_interesting
    }
}

impl Bug {
    pub fn decr_index(&mut self) {
        self.index -= 1;
    }
}

impl Reducer {
    pub fn create(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        cmdi: Box<dyn CommandInterface>,
        supervisor: Box<dyn Supervisor>,
    ) -> anyhow::Result<Self> {
        let runner = Runner::create(
            fst_mount,
            snd_mount,
            crashes_path,
            config,
            false,
            cmdi,
            supervisor,
            (vec![], vec![]),
        )
        .with_context(|| "failed to create runner")?;
        Ok(Self { runner })
    }

    pub fn run(&mut self, test_path: &LocalPath, save_to_dir: &LocalPath) -> anyhow::Result<()> {
        info!("read testcase at '{}'", test_path);
        let input = read_to_string(test_path).with_context(|| "failed to read testcase")?;
        let input: Workload =
            serde_json::from_str(&input).with_context(|| "failed to parse json")?;

        let binary_path = self.runner.compile_test(&input)?;

        match self.runner.run_harness(&binary_path)? {
            (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                let diffs_info = self.get_running_results(&fst_outcome, &snd_outcome)?;

                if diffs_info.has_some_interesting() {
                    let index = input.ops.len() - 1;
                    self.reduce(
                        &mut vec![Bug {
                            name: "original".to_string(),
                            dash_bug: diffs_info.dash_diff,
                            trace_bug: diffs_info.trace_diff,
                            workload: input,
                            index,
                        }],
                        save_to_dir,
                    )?;
                } else {
                    warn!("crash not detected");
                }
            }
            _ => todo!("handle all outcomes"),
        };

        Ok(())
    }

    fn reduce(&mut self, mut bugs: &mut Vec<Bug>, output_dir: &LocalPath) -> anyhow::Result<()> {
        info!("reduce using hash difference");
        let mut all_bugs_reduced = false;

        while !all_bugs_reduced {
            all_bugs_reduced = true;
            let mut new_bugs: Vec<Bug> = vec![];

            for bug_index in 0..bugs.len() {
                let bug = bugs.get(bug_index).unwrap().clone();
                if bug.index < 0 {
                    continue;
                }
                all_bugs_reduced = false;
                if let Some(reduced) = remove(&bug.workload, bug.index) {
                    let binary_path = self.runner.compile_test(&reduced)?;

                    match self.runner.run_harness(&binary_path)? {
                        (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                            let diffs_info =
                                self.get_running_results(&fst_outcome, &snd_outcome)?;
                            if !diffs_info.has_some_interesting() {
                                continue;
                            }

                            let matched_name =
                                self.find_match_bug_and_update(&diffs_info, &bug, bugs, &reduced);

                            let bug_name = match matched_name {
                                Some(name) => name,
                                None => {
                                    let new_bug = self.create_new_bug(&diffs_info, &bug, &reduced);
                                    let name = new_bug.name.clone();
                                    new_bugs.push(new_bug);
                                    name
                                }
                            };

                            self.runner.report_diff(
                                &reduced,
                                format!("{}/{}", bug_name, bug_index),
                                &binary_path,
                                output_dir.clone(),
                                diffs_info.dash_diff,
                                &fst_outcome,
                                &snd_outcome,
                                "".to_owned(),
                            )?;
                        }
                        _ => todo!("handle all outcomes"),
                    };
                }
                if bug.index >= 0 {
                    bugs.get_mut(bug_index).unwrap().decr_index();
                }
            }
            bugs.append(&mut new_bugs);
        }
        Ok(())
    }

    fn find_match_bug_and_update(
        &mut self,
        diffs_info: &RunningResults,
        init_bug: &Bug,
        bugs: &mut Vec<Bug>,
        reduced: &Workload,
    ) -> Option<String> {
        for b_i in 0..bugs.len() {
            let matched_bug = bugs.get_mut(b_i).unwrap();
            if matched_bug.trace_bug == diffs_info.trace_diff
                && matched_bug.dash_bug == diffs_info.dash_diff
            {
                matched_bug.workload = reduced.clone();
                if matched_bug.name != init_bug.name {
                    matched_bug.index = init_bug.index - 1;
                }
                return Some(matched_bug.name.clone());
            }
        }
        return None;
    }

    fn create_new_bug(
        &mut self,
        diffs_info: &RunningResults,
        init_bug: &Bug,
        reduced: &Workload,
    ) -> Bug {
        return Bug {
            name: format!("{}-{}", init_bug.name.clone(), init_bug.index.clone()),
            workload: reduced.clone(),
            dash_bug: diffs_info.dash_diff.clone(),
            trace_bug: diffs_info.trace_diff.clone(),
            index: init_bug.index - 1,
        };
    }

    fn get_running_results(
        &mut self,
        fst_outcome: &Completed,
        snd_outcome: &Completed,
    ) -> anyhow::Result<RunningResults> {
        let fst_trace =
            parse_trace(&fst_outcome.dir).with_context(|| "failed to parse first trace")?;
        let snd_trace = parse_trace(&snd_outcome.dir)
            .with_context(|| "failed to parse second trace")?;

        let dash_interesting = self
            .runner
            .dash_objective
            .is_interesting()
            .with_context(|| "failed to do dash objective")?;
        let trace_interesting = self
            .runner
            .trace_objective
            .is_interesting(&fst_trace, &snd_trace)
            .with_context(|| "failed to do trace objective")?;

        let dash_diff = if dash_interesting {
            self.runner
                .dash_objective
                .get_diff()
        } else {
            vec![]
        };

        let trace_diff = if trace_interesting {
            self.runner
                .trace_objective
                .get_diff(&fst_trace, &snd_trace)
        } else {
            vec![]
        };
        return Ok(RunningResults {
            dash_interesting,
            trace_interesting,
            dash_diff,
            trace_diff,
        });
    }
}
