/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::abstract_fs::workload::Workload;

use super::feedback::InputCoverage;

pub struct Seed {
    pub workload: Workload,
    pub times_choosen: u64,
    pub fst_coverage: InputCoverage,
    pub snd_coverage: InputCoverage,
}

impl Seed {
    pub fn new(
        workload: Workload,
        fst_coverage: InputCoverage,
        snd_coverage: InputCoverage,
    ) -> Self {
        Self {
            workload,
            times_choosen: 0,
            fst_coverage,
            snd_coverage,
        }
    }
}
