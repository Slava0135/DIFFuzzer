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
