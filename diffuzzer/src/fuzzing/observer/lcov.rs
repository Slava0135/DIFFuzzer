/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

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
