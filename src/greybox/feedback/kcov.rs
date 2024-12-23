use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use log::debug;

pub struct KCovFeedback {
    all_coverage: HashSet<u64>,
    kcov_path: Box<Path>,
}

impl KCovFeedback {
    pub fn new(kcov_path: Box<Path>) -> Self {
        Self {
            all_coverage: HashSet::new(),
            kcov_path,
        }
    }
    pub fn is_interesting(&mut self) -> anyhow::Result<bool>{
        debug!("do kcov feedback");
        let kcov = File::open(self.kcov_path.as_ref())?;
        let reader = BufReader::new(kcov);
        let mut new_coverage = HashSet::new();
        for line in reader.lines() {
            let addr = line?;
            let addr = parse_addr(addr)?;
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

fn parse_addr(addr: String) -> Result<u64, std::num::ParseIntError> {
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
            parse_addr("0xffffffff81460712".to_owned()).unwrap()
        );
    }
}
