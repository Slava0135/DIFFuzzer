use std::{fs, marker::PhantomData, path::Path, process::Command};

use libafl::{
    executors::{Executor, ExitKind},
    inputs::UsesInput,
    state::{HasExecutions, State, UsesState},
};

use crate::{
    abstract_fs::{encode::encode_c, types::Workload},
    fs_wrap::{setup, teardown, FileSystemType},
};

struct WorkloadExecutor<S: State> {
    phantom: PhantomData<S>,
    fs_type: FileSystemType,
    fs_dir: Box<Path>,
    test_dir: Box<Path>,
}

impl<S: State> WorkloadExecutor<S> {
    pub fn new(
        _state: &S,
        fs_type: FileSystemType,
        fs_dir: Box<Path>,
        test_dir: Box<Path>,
    ) -> Self {
        Self {
            phantom: PhantomData,
            fs_type,
            fs_dir,
            test_dir,
        }
    }
}

impl<S: State> UsesState for WorkloadExecutor<S> {
    type State = S;
}

impl<EM, S, Z> Executor<EM, Z> for WorkloadExecutor<S>
where
    EM: UsesState<State = S>,
    S: State + HasExecutions + UsesInput<Input = Workload>,
    Z: UsesState<State = S>,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        _state: &mut Self::State,
        _mgr: &mut EM,
        input: &Self::Input,
    ) -> Result<ExitKind, libafl::Error> {
        let encoded = encode_c(input.clone());
        let test_path = self.test_dir.join("test.c");
        let test_exec = self.test_dir.join("test.out");
        fs::write(test_path, encoded)?;
        let mut make = Command::new("make");
        make.arg("-C").arg(self.test_dir.as_os_str());
        make.output()?;

        setup(&self.fs_dir, self.fs_type)?;
        let mut exec = Command::new(format!("./{}", test_exec.display()));
        exec.arg(self.fs_dir.as_ref());
        let output = exec.output()?;
        teardown(&self.fs_dir, self.fs_type)?;

        if output.status.success() {
            Ok(ExitKind::Ok)
        } else {
            Ok(ExitKind::Crash)
        }
    }
}
