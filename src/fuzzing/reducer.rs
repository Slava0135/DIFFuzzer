use std::{fs::read_to_string, path::Path};

use anyhow::{Context, Ok};
use log::info;

use crate::{
    abstract_fs::workload::Workload, config::Config, fuzzing::common::parse_trace,
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

        Ok(())
    }
}
