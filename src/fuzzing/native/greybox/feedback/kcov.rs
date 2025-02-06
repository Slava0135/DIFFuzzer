use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
};

use anyhow::Context;
use log::debug;

use crate::path::RemotePath;

pub const KCOV_FILENAME: &str = "kcov.dat";

pub struct KCovFeedback {
    all_coverage: HashSet<u64>,
    kcov_path: RemotePath,
}

impl KCovFeedback {
    pub fn new(kcov_path: RemotePath) -> Self {
        Self {
            all_coverage: HashSet::new(),
            kcov_path,
        }
    }
    pub fn is_interesting(&mut self) -> anyhow::Result<bool> {
        todo!("use cmdi");
        debug!("do kcov feedback");
        let kcov = File::open(&self.kcov_path.base)
            .with_context(|| format!("failed to open kcov file at '{}'", self.kcov_path))?;
        let reader = BufReader::new(kcov);
        let mut new_coverage = HashSet::new();
        for line in reader.lines() {
            let addr = line.with_context(|| format!("failed to read lines from kcov file"))?;
            let addr = parse_addr(&addr)
                .with_context(|| format!("failed to parse addr from kcov line '{}'", addr))?;
            new_coverage.insert(addr);
        }
        let c = self.all_coverage.clone();
        let diff: Vec<&u64> = new_coverage.difference(&c).collect();
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

fn parse_addr(addr: &str) -> Result<u64, std::num::ParseIntError> {
    let prefix_removed = addr.trim_start_matches("0x");
    u64::from_str_radix(prefix_removed, 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_addr() {
        assert_eq!(
            18446744071583434514,
            parse_addr("0xffffffff81460712").unwrap()
        );
    }
}
