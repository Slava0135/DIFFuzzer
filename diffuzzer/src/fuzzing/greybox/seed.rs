use std::collections::HashSet;

use crate::abstract_fs::workload::Workload;

pub struct Seed {
    pub workload: Workload,
    pub times_choosen: u64,
    pub coverage: HashSet<u64>,
}

impl Seed {
    pub fn new(workload: Workload, coverage: HashSet<u64>) -> Self {
        Self {
            workload,
            times_choosen: 0,
            coverage,
        }
    }
}
