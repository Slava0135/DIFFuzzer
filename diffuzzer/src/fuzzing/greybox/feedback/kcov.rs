/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    collections::{HashMap, HashSet},
    fs,
};

use anyhow::Context;
use log::debug;

use crate::{fuzzing::outcome::Completed, path::LocalPath};

use super::{CoverageFeedback, CoverageMap, CoverageType, FeedbackOpinion, InputCoverage};

pub const KCOV_FILENAME: &str = "kcov.dat";

pub struct KCovCoverageFeedback {
    map: CoverageMap,
}

impl KCovCoverageFeedback {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl CoverageFeedback for KCovCoverageFeedback {
    fn coverage_type(&self) -> CoverageType {
        CoverageType::KCov
    }
    fn map(&self) -> &CoverageMap {
        &self.map
    }
    fn opinion(&mut self, outcome: &Completed) -> anyhow::Result<FeedbackOpinion> {
        debug!("do kcov feedback");
        let new_coverage = parse_kcov(&outcome.dir)?;
        let mut is_interesting = false;
        for addr in &new_coverage {
            let count = self.map.get(addr).unwrap_or(&0);
            if *count == 0 {
                is_interesting = true;
            }
            self.map.insert(*addr, *count + 1);
        }
        if is_interesting {
            Ok(FeedbackOpinion::Interesting(new_coverage))
        } else {
            Ok(FeedbackOpinion::NotInteresting(new_coverage))
        }
    }
}

fn parse_kcov(dir: &LocalPath) -> anyhow::Result<InputCoverage> {
    let path = dir.join(KCOV_FILENAME);
    let kcov = fs::read_to_string(&path)
        .with_context(|| format!("failed to read kcov file at {}", path))?;
    let mut coverage = HashSet::new();
    for line in kcov.lines() {
        let addr = parse_addr(line)
            .with_context(|| format!("failed to parse addr from kcov line '{}'", line))?;
        coverage.insert(addr);
    }
    Ok(coverage)
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
