/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct LCovTrace {
    pub files: HashMap<String, LCovTraceOneFile>,
}

impl LCovTrace {
    pub fn parse_from(data: &str) -> Self {
        let mut lcov = LCovTrace::new();
        let mut current_file: Option<String> = None;
        let mut trace = LCovTraceOneFile::new();
        for line in data.lines() {
            if let Some(line) = LCovLine::parse_from(line) {
                match line {
                    LCovLine::SourceFileName(next_file) => match current_file.clone() {
                        None => current_file = Some(next_file),
                        _ => {}
                    },
                    LCovLine::LineExecutionCount(line, count) => {
                        trace.add_line(line, count);
                    }
                    LCovLine::EndOfRecord() => match current_file.clone() {
                        Some(file) => {
                            let old = trace;
                            trace = LCovTraceOneFile::new();
                            lcov.add_file(file, old);
                            current_file = None;
                        }
                        _ => {}
                    },
                }
            }
        }
        lcov
    }
    fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
    fn add_file(&mut self, name: String, file: LCovTraceOneFile) {
        self.files.insert(name, file);
    }
}

enum LCovLine {
    SourceFileName(String),
    LineExecutionCount(u32, u64),
    EndOfRecord(),
}

impl LCovLine {
    fn parse_from(line: &str) -> Option<Self> {
        let line = line.trim();
        if line == "end_of_record" {
            return Some(LCovLine::EndOfRecord());
        }
        if let Some((tag, data)) = line.split_once(':') {
            match tag {
                "SF" => return Some(LCovLine::SourceFileName(data.to_owned())),
                "DA" => {
                    let segments: Vec<&str> = data.split(',').collect();
                    if segments.len() >= 2 {
                        match (segments[0].parse::<u32>(), segments[1].parse::<u64>()) {
                            (Ok(line), Ok(count)) => {
                                return Some(LCovLine::LineExecutionCount(line, count));
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LCovTraceOneFile {
    pub coverage_map: HashMap<u32, u64>,
}

impl LCovTraceOneFile {
    fn new() -> Self {
        Self {
            coverage_map: HashMap::new(),
        }
    }
    fn add_line(&mut self, line: u32, count: u64) {
        self.coverage_map.insert(line, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lcov() {
        let data = r#"
TN:
SF:/root/littlefs-fuse/littlefs/lfs_util.c
FN:17,lfs_crc
FNDA:65,lfs_crc
DA:17,65
DA:25,65
DA:27,317
DA:28,252
DA:29,252
DA:32,65
end_of_record
TN:
SF:/root/littlefs-fuse/littlefs/lfs_util.h
FN:126,lfs_max
FNDA:42,lfs_max
FN:130,lfs_min
FNDA:143,lfs_min
FN:135,lfs_aligndown
FNDA:17,lfs_aligndown
FN:139,lfs_alignup
FNDA:10,lfs_alignup
FN:144,lfs_npw2
FNDA:1,lfs_npw2
FN:161,lfs_ctz
FNDA:0,lfs_ctz
FN:170,lfs_popc
FNDA:0,lfs_popc
FN:182,lfs_scmp
FNDA:2,lfs_scmp
FN:187,lfs_fromle32
FNDA:18,lfs_fromle32
FN:205,lfs_tole32
FNDA:11,lfs_tole32
FN:210,lfs_frombe32
FNDA:18,lfs_frombe32
FN:228,lfs_tobe32
FNDA:6,lfs_tobe32
FN:245,lfs_malloc
FNDA:3,lfs_malloc
FN:257,lfs_free
FNDA:3,lfs_free
DA:126,42
DA:127,42
DA:130,143
DA:131,143
DA:135,17
DA:136,17
DA:139,10
DA:140,10
DA:144,1
DA:146,1
DA:161,0
DA:163,0
DA:170,0
DA:172,0
DA:182,2
DA:183,2
DA:187,18
DA:191,18
DA:205,11
DA:206,11
DA:210,18
DA:215,18
DA:228,6
DA:229,6
DA:245,3
DA:249,3
DA:257,3
DA:261,3
DA:265,3
end_of_record
"#
        .trim();
        let mut lfs_util_c_map = HashMap::new();
        lfs_util_c_map.insert(17, 65);
        lfs_util_c_map.insert(25, 65);
        lfs_util_c_map.insert(27, 317);
        lfs_util_c_map.insert(28, 252);
        lfs_util_c_map.insert(29, 252);
        lfs_util_c_map.insert(32, 65);
        let mut lfs_util_h_map = HashMap::new();
        lfs_util_h_map.insert(126, 42);
        lfs_util_h_map.insert(127, 42);
        lfs_util_h_map.insert(130, 143);
        lfs_util_h_map.insert(131, 143);
        lfs_util_h_map.insert(135, 17);
        lfs_util_h_map.insert(136, 17);
        lfs_util_h_map.insert(139, 10);
        lfs_util_h_map.insert(140, 10);
        lfs_util_h_map.insert(144, 1);
        lfs_util_h_map.insert(146, 1);
        lfs_util_h_map.insert(161, 0);
        lfs_util_h_map.insert(163, 0);
        lfs_util_h_map.insert(170, 0);
        lfs_util_h_map.insert(172, 0);
        lfs_util_h_map.insert(182, 2);
        lfs_util_h_map.insert(183, 2);
        lfs_util_h_map.insert(187, 18);
        lfs_util_h_map.insert(191, 18);
        lfs_util_h_map.insert(205, 11);
        lfs_util_h_map.insert(206, 11);
        lfs_util_h_map.insert(210, 18);
        lfs_util_h_map.insert(215, 18);
        lfs_util_h_map.insert(228, 6);
        lfs_util_h_map.insert(229, 6);
        lfs_util_h_map.insert(245, 3);
        lfs_util_h_map.insert(249, 3);
        lfs_util_h_map.insert(257, 3);
        lfs_util_h_map.insert(261, 3);
        lfs_util_h_map.insert(265, 3);
        let lfs_util_c = LCovTraceOneFile {
            coverage_map: lfs_util_c_map,
        };
        let lfs_util_h = LCovTraceOneFile {
            coverage_map: lfs_util_h_map,
        };
        let mut expected = LCovTrace::new();
        expected.add_file(
            "/root/littlefs-fuse/littlefs/lfs_util.c".to_owned(),
            lfs_util_c,
        );
        expected.add_file(
            "/root/littlefs-fuse/littlefs/lfs_util.h".to_owned(),
            lfs_util_h,
        );
        let actual = LCovTrace::parse_from(&data);
        assert_eq!(expected, actual);
    }
}
