/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs::read_to_string;

use anyhow::{Context, Ok};
use log::{info, warn};

use crate::fuzzing::outcome::Completed;
use crate::path::RemotePath;
use crate::{
    abstract_fs::{mutator::remove, workload::Workload},
    command::CommandInterface,
    config::Config,
    fuzzing::{outcome::Outcome, runner::parse_trace},
    mount::FileSystemMount,
    path::LocalPath,
    supervisor::Supervisor,
};

use super::runner::{Runner, RunningResults};

pub struct Reducer {
    runner: Runner,
    limit_bugs: usize,
}

#[derive(Clone)]
pub struct Bug {
    name: String,
    workload: Workload,
    diffs: RunningResults,
    index: usize,
    reduced: bool,
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
        Ok(Self { runner, limit_bugs })
    }

    pub fn run(&mut self, test_path: &LocalPath, save_to_dir: &LocalPath) -> anyhow::Result<()> {
        info!("read testcase at '{}'", test_path);
        let input = read_to_string(test_path).with_context(|| "failed to read testcase")?;
        let input: Workload =
            serde_json::from_str(&input).with_context(|| "failed to parse json")?;

        let binary_path = self.runner.compile_test(&input)?;

        match self.runner.run_harness(&binary_path)? {
            (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => {
                let diffs = self
                    .runner
                    .get_running_results(&fst_outcome, &snd_outcome)?;

                if diffs.has_some_interesting() {
                    let index = input.ops.len() - 1;
                    self.reduce(
                        &mut vec![Bug {
                            name: "original".to_string(),
                            diffs,
                            workload: input,
                            index,
                            reduced: false,
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

    fn reduce(&mut self, bugs: &mut Vec<Bug>, output_dir: &LocalPath) -> anyhow::Result<()> {
        info!("reduce testcase");
        let mut all_bugs_reduced = false;

        while !all_bugs_reduced {
            all_bugs_reduced = true;
            let mut new_bugs: Vec<Bug> = vec![];

            for bug_index in 0..bugs.len() {
                let bug = bugs.get(bug_index).unwrap().clone();
                if bug.reduced {
                    continue;
                }
                all_bugs_reduced = false;

                if let Some(reduced) = remove(&bug.workload, bug.index) {
                    let binary_path = self.runner.compile_test(&reduced)?;

                    match self.runner.run_harness(&binary_path)? {
                        (Outcome::Completed(fst_outcome), Outcome::Completed(snd_outcome)) => self
                            .completed_handler(
                                fst_outcome,
                                snd_outcome,
                                &bug,
                                bugs,
                                &mut new_bugs,
                                reduced,
                                output_dir,
                                binary_path,
                            )?, // todo: simplify
                        _ => todo!("handle all outcomes"),
                    };
                }
                if bug.index > 0 {
                    bugs.get_mut(bug_index).unwrap().index -= 1;
                } else {
                    bugs.get_mut(bug_index).unwrap().reduced = true;
                }
            }
            bugs.append(&mut new_bugs);
        }
        Ok(())
    }

    fn completed_handler(
        &mut self,
        fst_outcome: Completed,
        snd_outcome: Completed,
        init_bug: &Bug,
        bugs: &mut Vec<Bug>,
        new_bugs: &mut Vec<Bug>,
        reduced_workload: Workload,
        output_dir: &LocalPath,
        binary_path: RemotePath,
    ) -> anyhow::Result<()> {
        let diffs_info = self
            .runner
            .get_running_results(&fst_outcome, &snd_outcome)?;
        if !diffs_info.has_some_interesting() {
            return Ok(());
        }

        let matched_name =
            self.find_match_bug_and_update(&diffs_info, init_bug, bugs, &reduced_workload);

        let bug_name = match matched_name {
            Some(name) => name,
            None => {
                if self.limit_reached(bugs.len(), new_bugs.len()) {
                    return Ok(());
                }

                let new_bug = self.create_new_bug(&diffs_info, init_bug, &reduced_workload);
                let name = new_bug.name.clone();
                new_bugs.push(new_bug);
                name
            }
        };

        self.runner.report_diff(
            &reduced_workload,
            format!("{}/{}", bug_name, init_bug.index),
            &binary_path,
            output_dir.clone(),
            diffs_info.dash_diff,
            &fst_outcome,
            &snd_outcome,
            "".to_owned(),
        )?;

        return Ok(());
    }

    fn find_match_bug_and_update(
        &mut self,
        diffs_info: &RunningResults,
        init_bug: &Bug,
        bugs: &mut Vec<Bug>,
        reduced_workload: &Workload,
    ) -> Option<String> {
        for b_i in 0..bugs.len() {
            let matched_bug = bugs.get_mut(b_i).unwrap();
            if *diffs_info == matched_bug.diffs {
                matched_bug.workload = reduced_workload.clone();
                if matched_bug.name != init_bug.name {
                    matched_bug.reduced = init_bug.index == 0;
                    matched_bug.index = if matched_bug.reduced {
                        0
                    } else {
                        init_bug.index - 1
                    };
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
        reduced_workload: &Workload,
    ) -> Bug {
        let reduced = init_bug.index == 0;
        let index = if reduced { 0 } else { init_bug.index - 1 };

        return Bug {
            name: format!("{}-{}", init_bug.name.clone(), init_bug.index.clone()),
            workload: reduced_workload.clone(),
            diffs: diffs_info.clone(),
            index,
            reduced,
        };
    }

    fn limit_reached(&self, bugs_len: usize, new_bugs_len: usize) -> bool {
        self.limit_bugs > 0 && self.limit_bugs <= (bugs_len + new_bugs_len)
    }
}
