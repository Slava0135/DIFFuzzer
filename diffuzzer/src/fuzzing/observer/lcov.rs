use crate::{command::CommandInterface, path::RemotePath};

use super::Observer;

pub struct LCovObserver {}

impl Observer for LCovObserver {
    fn pre_exec(&mut self, cmdi: &dyn CommandInterface, output_dir: &RemotePath) -> anyhow::Result<()> {
        todo!()
    }

    fn post_exec(&mut self, cmdi: &dyn CommandInterface, output_dir: &RemotePath) -> anyhow::Result<()> {
        todo!()
    }

    fn skip_exec(&mut self) {}
}

impl LCovObserver {
    pub fn new() -> Self {
        Self {}
    }
}
