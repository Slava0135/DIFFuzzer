/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    collections::{HashMap, HashSet},
    fs,
};

use anyhow::Context;
use log::debug;

use crate::path::LocalPath;

use super::{CoverageMap, InputCoverage};

pub const KCOV_FILENAME: &str = "kcov.dat";

pub struct KCovFeedback {
    coverage_map: CoverageMap,
}

impl KCovFeedback {
    pub fn new() -> Self {
        Self {
            coverage_map: HashMap::new(),
        }
    }
    pub fn is_interesting(&mut self, coverage: &InputCoverage) -> bool {
        debug!("do kcov feedback");
        let old = self.coverage_map.keys().copied().collect();
        let diff: Vec<&u64> = coverage.difference(&old).collect();
        !diff.is_empty()
    }
    pub fn update_map(&mut self, coverage: &InputCoverage) {
        for addr in coverage {
            let count = self.coverage_map.get(addr).unwrap_or(&0);
            self.coverage_map.insert(*addr, *count + 1);
        }
    }
    pub fn map(&self) -> &CoverageMap {
        &self.coverage_map
    }
}

pub fn parse_kcov(dir: &LocalPath) -> anyhow::Result<InputCoverage> {
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
