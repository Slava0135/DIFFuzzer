use std::{borrow::Cow, fs::read_to_string, path::Path};

use libafl::{executors::ExitKind, inputs::UsesInput, observers::Observer};
use libafl_bolts::{ErrorBacktrace, Named};
use serde::{Deserialize, Serialize};

use crate::abstract_fs::trace::{Trace, TraceError};

#[derive(Debug, Deserialize, Serialize)]
pub struct TraceObserver {
    pub trace: Option<Trace>,
    trace_path: Box<Path>,
}

impl TraceObserver {
    pub fn new(trace_path: Box<Path>) -> TraceObserver {
        TraceObserver {
            trace: None,
            trace_path,
        }
    }
}

impl From<TraceError> for libafl::Error {
    fn from(e: TraceError) -> Self {
        libafl::Error::IllegalArgument(format!("{:?}", e), ErrorBacktrace::new())
    }
}

impl<S> Observer<S::Input, S> for TraceObserver
where
    S: UsesInput,
{
    fn flush(&mut self) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn pre_exec(&mut self, _state: &mut S, _input: &S::Input) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn post_exec(
        &mut self,
        _state: &mut S,
        _input: &S::Input,
        _exit_kind: &ExitKind,
    ) -> Result<(), libafl::Error> {
        self.trace = None;
        self.trace = Some(Trace::try_parse(read_to_string(self.trace_path.as_ref())?)?);
        Ok(())
    }

    fn pre_exec_child(
        &mut self,
        _state: &mut S,
        _input: &<S as UsesInput>::Input,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }

    fn post_exec_child(
        &mut self,
        _state: &mut S,
        _input: &<S as UsesInput>::Input,
        _exit_kind: &ExitKind,
    ) -> Result<(), libafl::Error> {
        Ok(())
    }
}

impl Named for TraceObserver {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &Cow::Borrowed("TraceObserver")
    }
}
