/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{collections::HashSet, fs};

use anyhow::Context;
use log::debug;

use crate::fuzzing::outcome::Outcome;

pub const KCOV_FILENAME: &str = "kcov.dat";

pub struct KCovFeedback {
    all_coverage: HashSet<u64>,
}

impl KCovFeedback {
    pub fn new() -> Self {
        Self {
            all_coverage: HashSet::new(),
        }
    }
    pub fn is_interesting(&mut self, outcome: &Outcome) -> anyhow::Result<bool> {
        debug!("do kcov feedback");
        let path = outcome.dir.join(KCOV_FILENAME);
        let kcov = fs::read_to_string(&path)
            .with_context(|| format!("failed to read kcov file at {}", path))?;
        let mut new_coverage = HashSet::new();
        for line in kcov.lines() {
            let addr = parse_addr(line)
                .with_context(|| format!("failed to parse addr from kcov line '{}'", line))?;
            new_coverage.insert(addr);
        }
        let c = self.all_coverage.clone();
        let diff: Vec<&u64> = new_coverage.difference(&c).collect();
        if diff.is_empty() {
            Ok(false)
        } else {
            for v in diff {
                self.all_coverage.insert(*v);
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
