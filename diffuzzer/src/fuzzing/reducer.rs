/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs::read_to_string;
use std::ops::Index;
use std::rc::Rc;
use std::borrow::Borrow;
use std::thread::yield_now;

use anyhow::{Context, Ok};
use dash::FileDiff;
use log::{info, warn};

use crate::abstract_fs::trace::TraceRow;
use crate::{
    abstract_fs::{mutator::remove, workload::Workload},
    command::CommandInterface,
    config::Config,
    fuzzing::{outcome::Outcome, runner::parse_trace},
    mount::FileSystemMount,
    path::LocalPath,
    supervisor::Supervisor,
};
use crate::fuzzing::objective::trace::TraceDiff;

use super::runner::Runner;

pub struct Reducer {
    runner: Runner,
}

pub struct Bug {
    name: String,
    workload: Workload,
    dash_bug: Vec<FileDiff>,
    trace_bug: Vec<TraceDiff>,
    index: usize,
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
                let fst_trace =
                    parse_trace(&fst_outcome.dir).with_context(|| "failed to parse first trace")?;
                let snd_trace = parse_trace(&snd_outcome.dir)
                    .with_context(|| "failed to parse second trace")?;
                let dash_diff_interesting = self
                    .runner
                    .dash_objective
                    .is_interesting()
                    .with_context(|| "failed to do hash objective")?;
                let trace_interesting = self
                    .runner
                    .trace_objective
                    .is_interesting(&fst_trace, &snd_trace)
                    .with_context(|| "failed to do trace objective")?;

                let new_dash_diff = if dash_diff_interesting {
                    self
                        .runner
                        .dash_objective
                        .get_diff()
                } else {
                    vec![]
                };

                let new_trace_diff = if trace_interesting {
                    self
                        .runner
                        .trace_objective
                        .get_diff(&fst_trace, &snd_trace)
                } else { vec![] };

                if dash_diff_interesting || trace_interesting {
                    let index = input.ops.len() - 1;
                    self.reduce(
                        &mut vec![Bug {
                            name: "original".to_string(),
                            dash_bug: new_dash_diff,
                            trace_bug: new_trace_diff,
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

    fn reduce(
        &mut self,
        mut bugs: &mut Vec<Bug>,
        output_dir: &LocalPath,
    ) -> anyhow::Result<()> {
        info!("reduce using hash difference");
        let mut bugs_reduced = false;

        while !bugs_reduced {
            bugs_reduced = true;
            let mut new_bugs: Vec<Bug> = vec![];
            for bug_index in 0..bugs.len() {
                let bug = bugs.get_mut(bug_index).unwrap();
                if bug.index < 0 { continue; }
                bugs_reduced = false;
                if let Some(reduced) = remove(&bug.workload, bug.index) {
                    let binary_path = self.runner.compile_test(&reduced)?;
                    match self.runner.run_harness(&binary_path)? {
                        (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                            let fst_trace =
                                parse_trace(&fst_outcome.dir).with_context(|| "failed to parse first trace")?;
                            let snd_trace = parse_trace(&snd_outcome.dir)
                                .with_context(|| "failed to parse second trace")?;

                            let hash_diff_interesting = self
                                .runner
                                .dash_objective
                                .is_interesting()
                                .with_context(|| "failed to do dash objective")?;
                            let trace_interesting = self
                                .runner
                                .trace_objective
                                .is_interesting(&fst_trace, &snd_trace)
                                .with_context(|| "failed to do trace objective")?;

                            let new_dash_diff = if hash_diff_interesting {
                                self
                                    .runner
                                    .dash_objective
                                    .get_diff()
                            } else {
                                vec![]
                            };

                            let new_trace_diff = if trace_interesting {
                                self
                                    .runner
                                    .trace_objective
                                    .get_diff(&fst_trace, &snd_trace)
                            } else { vec![] };

                            let mut option_bug = None;

                            for b_i in 0..bugs.len(){
                                let matched_bug = bugs.get_mut(b_i).unwrap();
                                if matched_bug.trace_bug == new_trace_diff && matched_bug.dash_bug == new_dash_diff {
                                    option_bug = Some(matched_bug);
                                    break;
                                }
                            }

                            match option_bug {
                                None => {
                                    new_bugs.push(Bug {
                                        name: format!("{}-{}", bug.name.clone(), bug.index.clone()),
                                        workload: reduced,
                                        dash_bug: new_dash_diff,
                                        trace_bug: new_trace_diff,
                                        index: bug.index - 1,
                                    })
                                }
                                Some(b) => {
                                    b.workload = reduced;
                                }
                            }
                        }

                        _ => {}
                    };
                }
                if bug.index >= 0 {
                    bug.decr_index();
                }
            }
            bugs.append(&mut new_bugs);
        }
        Ok(())
    }

    fn find_bug_by_diff<'a>(dash_diff: &Vec<FileDiff>, trace_diff: &Vec<TraceDiff>, bugs: &mut Vec<Bug>) -> Option<usize> {
        for (i, bug) in bugs.iter().enumerate() {
            if bug.trace_bug == *trace_diff && bug.dash_bug == *dash_diff {
                return Some(i);
            }
        }
        return None;
    }
}
