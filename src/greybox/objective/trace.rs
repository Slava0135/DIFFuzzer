use std::borrow::Cow;

use libafl::{
    feedbacks::{Feedback, StateInitializer},
    state::State,
    HasMetadata,
};
use libafl_bolts::{
    impl_serdeany,
    tuples::{Handle, MatchNameRef},
    Named,
};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{
    abstract_fs::{trace::Trace, types::Workload},
    greybox::observer::trace::TraceObserver,
};

pub struct TraceObjective {
    fst_observer: Handle<TraceObserver>,
    snd_observer: Handle<TraceObserver>,
    metadata: TraceMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct TraceMetadata {
    fst_trace: Option<Trace>,
    snd_trace: Option<Trace>,
}
impl_serdeany!(TraceMetadata);

impl TraceObjective {
    pub fn new(fst_observer: Handle<TraceObserver>, snd_observer: Handle<TraceObserver>) -> Self {
        Self {
            fst_observer,
            snd_observer,
            metadata: Default::default(),
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
        self.metadata = Default::default();
        self.metadata.fst_trace = observers
            .get(&self.fst_observer)
            .expect("failed to get first trace observer")
            .trace
            .clone();
        self.metadata.snd_trace = observers
            .get(&self.snd_observer)
            .expect("failed to get second trace observer")
            .trace
            .clone();
        match (&self.metadata.fst_trace, &self.metadata.snd_trace) {
            (Some(fst_trace), Some(snd_trace)) => Ok(!fst_trace.same_as(snd_trace)),
            (None, None) => Ok(false),
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
        testcase.metadata_map_mut().insert(self.metadata.clone());
        Ok(())
    }
}

impl Named for TraceObjective {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("TraceObjective")
    }
}
