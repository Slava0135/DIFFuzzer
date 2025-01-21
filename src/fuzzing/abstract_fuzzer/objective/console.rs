use anyhow::Context;
use log::debug;

use crate::{abstract_fs::output::Output, harness::ConsolePipe};

pub struct ConsoleObjective {
    fst_stdout: ConsolePipe,
    snd_stdout: ConsolePipe,
}

impl ConsoleObjective {
    pub fn new(fst_stdout: ConsolePipe, snd_stdout: ConsolePipe) -> Self {
        Self {
            fst_stdout,
            snd_stdout,
        }
    }
    pub fn is_interesting(&mut self) -> anyhow::Result<bool> {
        debug!("do console objective");
        let fst_output = Output::try_parse(&self.fst_stdout.borrow())
            .with_context(|| format!("failed to parse first stdout"))?;
        let snd_output = Output::try_parse(&self.snd_stdout.borrow())
            .with_context(|| format!("failed to parse second stdout"))?;
        Ok(fst_output.success_n != snd_output.success_n
            || fst_output.failure_n != snd_output.failure_n)
    }
}
