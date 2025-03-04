use crate::abstract_fs::workload::Workload;

pub struct Seed {
    pub workload: Workload,
}

impl Seed {
    pub fn new(workload: Workload) -> Self {
        Self { workload }
    }
}
