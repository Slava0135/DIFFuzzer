/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::{HashMap, VecDeque};
use std::fs::read_to_string;

use anyhow::{Context, Ok};
use log::{error, info, warn};
use thiserror::Error;

use crate::fuzzing::outcome::Completed;
use crate::path::RemotePath;
use crate::{
    abstract_fs::{mutator::remove, workload::Workload},
    command::CommandInterface,
    config::Config,
    fuzzing::outcome::Outcome,
    mount::FileSystemMount,
    path::LocalPath,
    supervisor::Supervisor,
};

use super::runner::{DiffOutcome, Runner};

pub struct Reducer {
    runner: Runner,
    limit_bugs: usize,
    limit_counter: usize,
    bugs: HashMap<DiffOutcome, Bug>,
    bugs_queue: VecDeque<DiffOutcome>,
}

#[derive(Error, Debug, PartialEq)]
pub enum ReducerError {
    #[error("Empty queue of bugs")]
    EmptyQueue,
    #[error("Hash map does not contain bug")]
    BugNotExists,
}

#[derive(Clone)]
pub struct Bug {
    name: String,
    workload: Workload,
    remove_pointer: usize,
}

impl Reducer {
    pub fn create(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
        crashes_path: LocalPath,
        cmdi: Box<dyn CommandInterface>,
        supervisor: Box<dyn Supervisor>,
        limit_bugs: usize,
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
        Ok(Self {
            runner,
            limit_bugs,
            limit_counter: 0,
            bugs: HashMap::new(),
            bugs_queue: VecDeque::new(),
        })
    }

    pub fn reset(&mut self) {
        self.limit_counter = 0;
        self.bugs = HashMap::new();
        self.bugs_queue = VecDeque::new();
    }

    pub fn run(&mut self, test_path: &LocalPath, save_to_dir: &LocalPath) -> anyhow::Result<()> {
        info!("read testcase at '{}'", test_path);
        self.reset();
        let input = read_to_string(test_path).with_context(|| "failed to read testcase")?;
        let input: Workload =
            serde_json::from_str(&input).with_context(|| "failed to parse json")?;

        let binary_path = self.runner.compile_test(&input)?;

        match self.runner.run_harness(&binary_path)? {
            (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                let diffs = self.runner.get_diffs(&fst_outcome, &snd_outcome)?;

                if diffs.has_some_interesting() {
                    let remove_pointer = input.ops.len() - 1;
                    self.bugs.insert(
                        diffs,
                        Bug {
                            name: "original".to_string(),
                            workload: input,
                            remove_pointer,
                        },
                    );
                    self.reduce(save_to_dir)?;
                } else {
                    warn!("crash not detected");
                }
            }
            _ => todo!("handle all outcomes"),
        };

        Ok(())
    }

    fn reduce(&mut self, output_dir: &LocalPath) -> anyhow::Result<()> {
        info!("reduce testcase");
        while !self.bugs_queue.is_empty() {
            let bug_diff = self
                .bugs_queue
                .pop_front()
                .ok_or(ReducerError::EmptyQueue)?;
            let bug = self
                .bugs
                .get(&bug_diff)
                .ok_or(ReducerError::BugNotExists)?
                .clone();

            if let Some(reduced) = remove(&bug.workload, bug.remove_pointer) {
                let binary_path = self.runner.compile_test(&reduced)?;

                match self.runner.run_harness(&binary_path)? {
                    (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => self
                        .completed_handler(
                            fst_outcome,
                            snd_outcome,
                            &bug,
                            reduced,
                            output_dir,
                            binary_path,
                        )?,
                    _ => todo!("handle all outcomes"),
                };
            }
            if bug.remove_pointer > 0 {
                self.bugs.get_mut(&bug_diff).unwrap().remove_pointer -= 1;
                self.bugs_queue.push_back(bug_diff);
            }
        }
        Ok(())
    }

    fn completed_handler(
        &mut self,
        fst_outcome: Completed,
        snd_outcome: Completed,
        init_bug: &Bug,
        reduced_workload: Workload,
        output_dir: &LocalPath,
        binary_path: RemotePath,
    ) -> anyhow::Result<()> {
        let diffs = self.runner.get_diffs(&fst_outcome, &snd_outcome)?;
        if !diffs.has_some_interesting() {
            return Ok(());
        }

        let matched_bug = self.bugs.get_mut(&diffs);

        let bug_name = match matched_bug {
            Some(bug) => {
                //todo: handle if len is equal or if workloads too different
                if reduced_workload.ops.len() >= bug.workload.ops.len() {
                    return Ok(());
                }
                bug.workload = reduced_workload.clone();
                if bug.name != init_bug.name {
                    if bug.remove_pointer > 0 {
                        bug.remove_pointer = init_bug.remove_pointer - 1
                    } else {
                        Self::remove_from_queue_by_value(&diffs, &mut self.bugs_queue);
                    };
                }
                bug.name.clone()
            }
            None => {
                if self.limit_reached() {
                    return Ok(());
                }
                self.limit_counter += 1;
                let new_bug = self.create_new_bug(init_bug, &reduced_workload);
                let name = new_bug.name.clone();
                self.bugs.insert(diffs.clone(), new_bug);
                self.bugs_queue.push_back(diffs.clone());
                name
            }
        };

        self.runner.report_diff(
            &reduced_workload,
            format!("{}/{}", bug_name, init_bug.remove_pointer),
            &binary_path,
            output_dir.clone(),
            diffs.dash_diff,
            &fst_outcome,
            &snd_outcome,
            "".to_owned(),
        )?;

        return Ok(());
    }

    fn create_new_bug(&mut self, init_bug: &Bug, reduced_workload: &Workload) -> Bug {
        let reduced = init_bug.remove_pointer == 0;
        let remove_pointer = if reduced {
            0
        } else {
            init_bug.remove_pointer - 1
        };

        return Bug {
            name: format!(
                "{}-{}",
                init_bug.name.clone(),
                init_bug.remove_pointer.clone()
            ),
            workload: reduced_workload.clone(),
            remove_pointer,
        };
    }

    fn limit_reached(&self) -> bool {
        self.limit_bugs != 0 && self.limit_counter >= self.limit_bugs
    }

    fn remove_from_queue_by_value(diffs: &DiffOutcome, bugs_queue: &mut VecDeque<DiffOutcome>) {
        if let Some(diff_index) = bugs_queue.iter().position(|d| *d == *diffs) {
            bugs_queue.remove(diff_index);
        }
    }
}
