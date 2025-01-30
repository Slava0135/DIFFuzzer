use std::{fs::read_to_string, path::Path};

use anyhow::{Context, Ok};
use log::{info, warn};

use crate::{
    abstract_fs::{mutator::remove, workload::Workload},
    config::Config,
    fuzzing::common::parse_trace,
    hasher::hasher::FileDiff,
    mount::mount::FileSystemMount,
};

use super::common::Runner;

pub struct Reducer {
    runner: Runner,
}

impl Reducer {
    pub fn new(
        config: Config,
        fst_mount: &'static dyn FileSystemMount,
        snd_mount: &'static dyn FileSystemMount,
    ) -> Self {
        Self {
            runner: Runner::new(fst_mount, snd_mount, config),
        }
    }

    pub fn run(&mut self, test_path: &Path, save_to_dir: &Path) -> anyhow::Result<()> {
        info!("running reducer");
        info!("reading testcase at '{}'", test_path.display());
        let input = read_to_string(test_path)
            .with_context(|| format!("failed to read testcase"))
            .unwrap();
        let input: Workload = serde_json::from_str(&input)
            .with_context(|| format!("failed to parse json"))
            .unwrap();

        let input_path = self.runner.compile_test(&input)?;

        self.runner.run_harness(&input_path)?;

        let fst_trace = parse_trace(&self.runner.fst_trace_path)
            .with_context(|| format!("failed to parse first trace"))?;
        let snd_trace = parse_trace(&self.runner.snd_trace_path)
            .with_context(|| format!("failed to parse second trace"))?;

        let hash_diff_interesting = self
            .runner
            .hash_objective
            .is_interesting()
            .with_context(|| format!("failed to do hash objective"))?;
        let trace_is_interesting = self
            .runner
            .trace_objective
            .is_interesting(&fst_trace, &snd_trace)
            .with_context(|| format!("failed to do trace objective"))?;

        if trace_is_interesting {
            todo!()
        } else if hash_diff_interesting {
            let old_diff = self.runner.hash_objective.get_diff();
            self.reduce_by_hash(input, old_diff, save_to_dir)?;
        } else {
            warn!("crash not detected");
        }

        Ok(())
    }

    fn reduce_by_hash(
        &mut self,
        input: Workload,
        old_diff: Vec<FileDiff>,
        save_to_dir: &Path,
    ) -> anyhow::Result<()> {
        info!("reducing using hash difference");
        let mut index = input.ops.len() - 1;
        let mut workload = input;
        loop {
            if let Some(reduced) = remove(&workload, index) {
                let input_path = self.runner.compile_test(&workload)?;
                self.runner.run_harness(&input_path)?;
                let hash_diff_interesting = self
                    .runner
                    .hash_objective
                    .is_interesting()
                    .with_context(|| format!("failed to do hash objective"))?;
                if hash_diff_interesting {
                    let new_diff = self.runner.hash_objective.get_diff();
                    if old_diff == new_diff {
                        workload = reduced;
                        info!("reduced workload (length = {})", workload.ops.len());
                        self.runner.report_crash(
                            &workload,
                            &input_path,
                            save_to_dir.to_path_buf().into_boxed_path(),
                            new_diff,
                        )?;
                    }
                }
            }
            if index == 0 {
                break;
            }
            index -= 1
        }
        Ok(())
    }
}
