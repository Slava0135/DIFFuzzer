use std::{fs::read_to_string, path::Path};

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
    fn is_interesting(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        debug!("do trace objective");
        let fst_trace = Trace::try_parse(read_to_string(self.fst_trace_path.as_ref())?)?;
        let snd_trace = Trace::try_parse(read_to_string(self.snd_trace_path.as_ref())?)?;
        Ok(!fst_trace.same_as(&snd_trace))
    }
}
