use std::{borrow::Cow, path::Path};

use libafl::{
    HasMetadata,
    feedbacks::{Feedback, StateInitializer},
    inputs::Input,
    state::State,
};
use libafl_bolts::{ErrorBacktrace, Named, tuples::MatchNameRef};

use crate::abstract_fs::{
    compile::{TEST_EXE_FILENAME, TEST_SOURCE_FILENAME},
    types::Workload,
};

pub struct SaveTestObjective {
    pub test_dir: Box<Path>,
    pub saved_test_dir: Box<Path>,
}

impl SaveTestObjective {
    pub fn new(test_dir: Box<Path>, saved_test_dir: Box<Path>) -> Self {
        Self {
            test_dir,
            saved_test_dir,
        }
    }
}

impl<S> StateInitializer<S> for SaveTestObjective {
    fn init_state(&mut self, _state: &mut S) -> Result<(), libafl::Error> {
        std::fs::create_dir(self.saved_test_dir.as_ref()).unwrap_or(());
        Ok(())
    }
}

impl<EM, OT, S> Feedback<EM, Workload, OT, S> for SaveTestObjective
where
    S: State,
    OT: MatchNameRef,
{
    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &Workload,
        _observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(true)
    }

    fn append_metadata(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _observers: &OT,
        testcase: &mut libafl::corpus::Testcase<Workload>,
    ) -> Result<(), libafl::Error> {
        let input = testcase.input().as_ref().unwrap().clone();
        testcase.metadata_map_mut().insert(input.clone());
        let name = input.generate_name(None);
        let path = self.saved_test_dir.join(name.clone());
        std::fs::copy(
            self.test_dir.join(TEST_SOURCE_FILENAME),
            path.with_extension("c"),
        )?;
        std::fs::copy(
            self.test_dir.join(TEST_EXE_FILENAME),
            path.with_extension("out"),
        )?;
        match serde_json::to_string_pretty(&input) {
            Ok(json) => std::fs::write(path.with_extension("json"), json)?,
            Err(err) => {
                return Err(libafl::Error::Serialize(
                    err.to_string(),
                    ErrorBacktrace::new(),
                ));
            }
        }
        Ok(())
    }
}

impl Named for SaveTestObjective {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("SaveTestObjective")
    }
}
