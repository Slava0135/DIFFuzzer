use std::{borrow::Cow, collections::HashSet};

use libafl::{
    feedbacks::{Feedback, StateInitializer},
    state::State,
};
use libafl_bolts::{
    tuples::{Handle, MatchNameRef},
    Named,
};

use crate::{abstract_fs::types::Workload, greybox::observer::kcov::KCovObserver};

pub struct KCovFeedback {
    observer: Handle<KCovObserver>,
    all_coverage: HashSet<u64>,
}

impl KCovFeedback {
    pub fn new(observer: Handle<KCovObserver>) -> Self {
        Self {
            observer,
            all_coverage: HashSet::new(),
        }
    }
}

impl<S> StateInitializer<S> for KCovFeedback {}

impl<EM, OT, S> Feedback<EM, Workload, OT, S> for KCovFeedback
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
        let coverage = &observers
            .get(&self.observer)
            .expect("failed to get kcov observer")
            .coverage;
        let c = self.all_coverage.clone();
        let diff: Vec<&u64> = coverage.difference(&c).collect();
        if diff.is_empty() {
            Ok(false)
        } else {
            for v in diff {
                self.all_coverage.insert(v.clone());
            }
            Ok(true)
        }
    }
}

impl Named for KCovFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("KCovFeedback")
    }
}
