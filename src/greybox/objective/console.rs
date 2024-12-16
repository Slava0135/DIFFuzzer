use std::{cell::RefCell, rc::Rc};

pub type ConsolePipe = Rc<RefCell<String>>;

use std::borrow::Cow;

use libafl::feedbacks::{Feedback, StateInitializer};
use libafl::state::State;
use libafl::HasMetadata;
use libafl_bolts::{impl_serdeany, Named};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::abstract_fs::output::Output;
use crate::abstract_fs::types::Workload;

pub struct ConsoleObjective {
    fst_stdout: ConsolePipe,
    fst_stderr: ConsolePipe,
    snd_stdout: ConsolePipe,
    snd_stderr: ConsolePipe,
    metadata: ConsoleMetadata,
}

impl ConsoleObjective {
    pub fn new(
        fst_stdout: ConsolePipe,
        fst_stderr: ConsolePipe,
        snd_stdout: ConsolePipe,
        snd_stderr: ConsolePipe,
    ) -> Self {
        Self {
            fst_stdout,
            fst_stderr,
            snd_stdout,
            snd_stderr,
            metadata: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct ConsoleMetadata {
    fst_stdout: String,
    fst_stderr: String,
    snd_stdout: String,
    snd_stderr: String,
}
impl_serdeany!(ConsoleMetadata);

impl<S> StateInitializer<S> for ConsoleObjective {}

impl<EM, OT, S> Feedback<EM, Workload, OT, S> for ConsoleObjective
where
    S: State,
{
    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &Workload,
        _observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        debug!("do console objective");
        let fst_output = Output::try_parse(self.fst_stdout.borrow().to_owned());
        let snd_output = Output::try_parse(self.snd_stdout.borrow().to_owned());
        match (fst_output, snd_output) {
            (Ok(fst_output), Ok(snd_output)) => Ok(fst_output.success_n != snd_output.success_n
                || fst_output.failure_n != snd_output.failure_n),
            (Err(_), Err(_)) => Ok(false),
            _ => Ok(true),
        }
    }

    fn append_metadata(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _observers: &OT,
        testcase: &mut libafl::corpus::Testcase<Workload>,
    ) -> Result<(), libafl::Error> {
        self.metadata.fst_stdout = self.fst_stdout.borrow().clone();
        self.metadata.fst_stderr = self.fst_stderr.borrow().clone();
        self.metadata.snd_stdout = self.snd_stdout.borrow().clone();
        self.metadata.snd_stderr = self.snd_stderr.borrow().clone();
        testcase.metadata_map_mut().insert(self.metadata.clone());
        Ok(())
    }
}

impl Named for ConsoleObjective {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("ConsoleObjective")
    }
}
