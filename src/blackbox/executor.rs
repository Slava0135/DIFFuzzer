use std::path::Path;
use std::process::{Command, Output};
use crate::blackbox::hasher_wrapper::compare_hash;

pub struct WorkloadExecutor {
    pub ref_path: Box<Path>,
    pub target_path: Box<Path>,
}

pub struct ExecResults<'ctx> {
    pub workload_executor: &'ctx WorkloadExecutor,
    pub output_ref: Output,
    pub output_target: Output,
}



impl WorkloadExecutor {
    pub fn execute_workload(&self, workload_path: Box<Path>) -> ExecResults {
        let exec = Command::new(format!("./{}", workload_path.display())).arg(&self.ref_path.as_ref());
        let output_ref = exec.output()?;
        let exec = Command::new(format!("./{}", workload_path.display())).arg(&self.target_path.as_ref());
        let output_target = exec.output()?;
        return ExecResults { workload_executor: &self, output_ref, output_target };
    }
}

impl ExecResults {
    pub fn compare_outputs(&self) {
        compare_hash(&self);
        //todo: compare traces
    }
}