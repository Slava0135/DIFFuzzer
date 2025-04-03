/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::num::ParseIntError;

use thiserror::Error;

/// Stores results of executing test workload operations.
#[derive(Debug, PartialEq, Clone)]
pub struct Trace {
    pub rows: Vec<TraceRow>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TraceRow {
    pub index: u32,
    command: String,
    return_code: i32,
    errno: Errno,
    extra: String,
    /// Original text of csv file corresponding to row
    source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TraceDiff {
    TraceRowIsDifferent { fst: TraceRow, snd: TraceRow },
    DifferentLength,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Errno {
    name: String,
    code: i32,
}

pub const TRACE_FILENAME: &str = "trace.csv";

type Result<T> = std::result::Result<T, TraceError>;

#[derive(Error, Debug, PartialEq)]
pub enum TraceError {
    #[error("invalid trace, must not be empty")]
    Empty,
    #[error("invalid column number")]
    InvalidColumnNumber,
    #[error("invalid integer format")]
    IntParse(ParseIntError),
    #[error("invalid errno string '{0}'")]
    InvalidErrno(String),
}

impl From<ParseIntError> for TraceError {
    fn from(err: ParseIntError) -> TraceError {
        TraceError::IntParse(err)
    }
}

impl Trace {
    pub fn try_parse(trace: String) -> Result<Trace> {
        let lines: Vec<&str> = trace.split('\n').collect();
        if lines.len() <= 1 {
            return Err(TraceError::Empty);
        }
        let mut trace = Trace { rows: vec![] };
        for line in &lines[1..] {
            if line.trim().is_empty() {
                break;
            }
            let columns: Vec<&str> = line.split(",").collect();
            if columns.len() != 5 {
                return Err(TraceError::InvalidColumnNumber);
            }
            let index = columns[0].trim().parse()?;
            let command = columns[1].trim().to_owned();
            let return_code = columns[2].trim().parse()?;
            let errno_string = columns[3].trim().to_owned();
            let extra = columns[4].trim().to_owned();
            let errno_parts: Vec<String> = errno_string
                .split(&['(', ')'])
                .map(|s| s.to_owned())
                .collect();
            let name = errno_parts
                .first()
                .ok_or(TraceError::InvalidErrno(errno_string.clone()))?
                .clone();
            let code: i32 = errno_parts
                .get(1)
                .ok_or(TraceError::InvalidErrno(errno_string.clone()))?
                .parse()?;

            trace.rows.push(TraceRow {
                index,
                command,
                return_code,
                errno: Errno { name, code },
                extra,
                source: line.trim_end().to_owned(),
            });
        }
        Ok(trace)
    }

    pub fn errors(&self) -> Vec<TraceRow> {
        self.rows
            .iter()
            .filter(|row| row.errno.code != 0)
            .cloned()
            .collect()
    }

    pub fn header() -> String {
        "Index,Command,ReturnCode,Errno,Extra".to_owned()
    }
}

impl TraceRow {
    pub fn ignore_index_equal(&self, other: &TraceRow) -> bool {
        self.command == other.command
            && self.return_code == other.return_code
            && self.extra == other.extra
            && self.errno == other.errno
    }
    pub fn source(&self) -> String {
        self.source.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_trace() {
        assert_eq!(Err(TraceError::Empty), Trace::try_parse("".to_owned()))
    }

    #[test]
    fn test_header_only() {
        assert_eq!(
            Ok(Trace { rows: vec![] }),
            Trace::try_parse("Index,Command,ReturnCode,Errno\n".to_owned())
        )
    }

    #[test]
    fn test_invalid_columns_count() {
        let trace = r#"
Index,Command,ReturnCode,Errno,Extra
    1,    Foo,        42,Success(0),a=1, ???
    2,    Bar,        -1,Error(42),b=2
"#
        .trim();
        assert_eq!(
            Err(TraceError::InvalidColumnNumber),
            Trace::try_parse(trace.to_owned())
        )
    }

    #[test]
    fn test_ok_trace() {
        let trace = r#"
Index,Command,ReturnCode,Errno,Extra
    1,    Foo,        42,Success(0),a=1
    2,    Bar,        -1,Error(42),b=2
"#
        .trim();
        assert_eq!(
            Ok(Trace {
                rows: vec![
                    TraceRow {
                        index: 1,
                        command: "Foo".to_owned(),
                        return_code: 42,
                        errno: Errno {
                            name: "Success".to_owned(),
                            code: 0,
                        },
                        extra: "a=1".to_owned(),
                        source: "    1,    Foo,        42,Success(0),a=1".to_owned()
                    },
                    TraceRow {
                        index: 2,
                        command: "Bar".to_owned(),
                        return_code: -1,
                        errno: Errno {
                            name: "Error".to_owned(),
                            code: 42,
                        },
                        extra: "b=2".to_owned(),
                        source: "    2,    Bar,        -1,Error(42),b=2".to_owned()
                    },
                ]
            }),
            Trace::try_parse(trace.to_owned())
        )
    }

    #[test]
    fn test_invalid_errno_no_brackets() {
        let trace = r#"
        Index,Command,ReturnCode,Errno,Extra
            1,    Foo,        42,Success 0,a=1
            2,    Bar,        -1,Error(42),b=2
        "#
        .trim();
        assert_eq!(
            Err(TraceError::InvalidErrno("Success 0".to_owned())),
            Trace::try_parse(trace.to_owned())
        )
    }
}
