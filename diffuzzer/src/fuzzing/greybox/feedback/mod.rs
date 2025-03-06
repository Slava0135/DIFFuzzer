/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

pub mod kcov;
pub mod lcov;

pub enum CoverageType {
    /// No coverage.
    None,
    /// Linux kernel coverage (use for kernel file systems).
    KCov,
    /// Coverage measurements on standard user space applications.
    LCov,
}

impl Display for CoverageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::KCov => write!(f, "KCov"),
            Self::LCov => write!(f, "LCov"),
        }
    }
}

pub enum FeedbackOpinion {
    Interesting(InputCoverage),
    NotInteresting(InputCoverage),
}

impl FeedbackOpinion {
    pub fn is_interesting(&self) -> bool {
        match self {
            Self::Interesting(_) => true,
            Self::NotInteresting(_) => false,
        }
    }
    pub fn coverage(self) -> InputCoverage {
        match self {
            Self::Interesting(coverage) => coverage,
            Self::NotInteresting(coverage) => coverage,
        }
    }
}

pub trait CoverageFeedback {
    fn coverage_type(&self) -> CoverageType;
    fn map(&self) -> &CoverageMap;
    fn opinion(&mut self) -> anyhow::Result<FeedbackOpinion>;
}

pub type InputCoverage = HashSet<u64>;

pub type CoverageMap = HashMap<u64, u64>;

pub struct DummyCoverageFeedback {
    map: CoverageMap,
}

impl DummyCoverageFeedback {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl CoverageFeedback for DummyCoverageFeedback {
    fn coverage_type(&self) -> CoverageType {
        CoverageType::None
    }
    fn map(&self) -> &CoverageMap {
        &self.map
    }
    fn opinion(&mut self) -> anyhow::Result<FeedbackOpinion> {
        Ok(FeedbackOpinion::NotInteresting(HashSet::new()))
    }
}

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
    fn opinion(&mut self) -> anyhow::Result<FeedbackOpinion> {
        Ok(FeedbackOpinion::NotInteresting(HashSet::new()))
    }
}

pub struct LCovCoverageFeedback {
    map: CoverageMap,
}

impl LCovCoverageFeedback {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl CoverageFeedback for LCovCoverageFeedback {
    fn coverage_type(&self) -> CoverageType {
        CoverageType::KCov
    }
    fn map(&self) -> &CoverageMap {
        &self.map
    }
    fn opinion(&mut self) -> anyhow::Result<FeedbackOpinion> {
        Ok(FeedbackOpinion::NotInteresting(HashSet::new()))
    }
}
