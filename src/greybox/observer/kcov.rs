use std::{
    borrow::Cow,
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use libafl::{executors::ExitKind, inputs::UsesInput, observers::Observer};
use libafl_bolts::Named;
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct KCovObserver {
    pub coverage: HashSet<u64>,
    kcov_path: Box<Path>,
}

impl KCovObserver {
    pub fn new(kcov_path: Box<Path>) -> KCovObserver {
        KCovObserver {
            coverage: HashSet::new(),
            kcov_path,
        }
    }
}

fn parse_addr(addr: String) -> Result<u64, std::num::ParseIntError> {
    let prefix_removed = addr.trim_start_matches("0x");
    u64::from_str_radix(prefix_removed, 16)
}

impl<S> Observer<S::Input, S> for KCovObserver
where
    S: UsesInput,
{
    fn post_exec(
        &mut self,
        _state: &mut S,
        _input: &S::Input,
        _exit_kind: &ExitKind,
    ) -> Result<(), libafl::Error> {
        debug!("observing kcov coverage");
        self.coverage.clear();
        let kcov = File::open(self.kcov_path.as_ref())?;
        let reader = BufReader::new(kcov);
        for line in reader.lines() {
            let addr = line?;
            let addr = parse_addr(addr)?;
            self.coverage.insert(addr);
        }
        Ok(())
    }
}

impl Named for KCovObserver {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &Cow::Borrowed("KCovObserver")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_addr() {
        assert_eq!(
            18446744071583434514,
            parse_addr("0xffffffff81460712".to_owned()).unwrap()
        );
    }
}
