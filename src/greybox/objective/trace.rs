use std::{fs::read_to_string, path::Path};

use anyhow::Context;
use log::debug;

use crate::abstract_fs::trace::Trace;

pub struct TraceObjective {
    fst_trace_path: Box<Path>,
    snd_trace_path: Box<Path>,
}

impl TraceObjective {
    pub fn new(fst_trace_path: Box<Path>, snd_trace_path: Box<Path>) -> Self {
        Self {
            fst_trace_path,
            snd_trace_path,
        }
    }
}

impl TraceObjective {
    pub fn is_interesting(&mut self) -> anyhow::Result<bool> {
        debug!("do trace objective");
        let fst_trace = read_to_string(&self.fst_trace_path).with_context(|| {
            format!(
                "failed to read trace at '{}'",
                self.fst_trace_path.display()
            )
        })?;
        let snd_trace = read_to_string(&self.snd_trace_path).with_context(|| {
            format!(
                "failed to read trace at '{}'",
                self.snd_trace_path.display()
            )
        })?;
        let fst_trace =
            Trace::try_parse(fst_trace).with_context(|| format!("failed to parse first trace"))?;
        let snd_trace =
            Trace::try_parse(snd_trace).with_context(|| format!("failed to parse second trace"))?;
        Ok(!fst_trace.same_as(&snd_trace))
    }
}
