use std::borrow::Cow;

use libafl::{
    feedbacks::{Feedback, StateInitializer},
    state::State,
};
use libafl_bolts::{
    tuples::{Handle, MatchNameRef},
    Named,
};
use log::debug;

use crate::{abstract_fs::types::Workload, greybox::observer::trace::TraceObserver};

pub struct TraceObjective {
    fst_observer: Handle<TraceObserver>,
    snd_observer: Handle<TraceObserver>,
}

impl TraceObjective {
    pub fn new(fst_observer: Handle<TraceObserver>, snd_observer: Handle<TraceObserver>) -> Self {
        Self {
            fst_observer,
            snd_observer,
        }
    }
}

impl<S> StateInitializer<S> for TraceObjective {}

impl<EM, OT, S> Feedback<EM, Workload, OT, S> for TraceObjective
where
    S: State,
    OT: MatchNameRef,
{
    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &Workload,
        observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        debug!("do trace objective");
        let fst_trace = observers
            .get(&self.fst_observer)
            .expect("failed to get first trace observer")
            .trace
            .clone();
        let snd_trace = observers
            .get(&self.snd_observer)
            .expect("failed to get second trace observer")
            .trace
            .clone();
        match (fst_trace, snd_trace) {
            (Some(fst_trace), Some(snd_trace)) => Ok(!fst_trace.same_as(snd_trace)),
            (None, None) => Ok(false),
            _ => Ok(true),
        }
    }
}

impl Named for TraceObjective {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("TraceObjective")
    }
}
