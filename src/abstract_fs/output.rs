use std::{error::Error, fmt::Display, num::ParseIntError};

use regex::Regex;

#[derive(Debug, PartialEq)]
pub struct Output {
    pub success_n: u16,
    pub failure_n: u16,
}

type Result<T> = std::result::Result<T, OutputError>;

#[derive(Debug, PartialEq)]
pub enum OutputError {
    Empty,
    Regex,
    IntParse(ParseIntError),
}

impl From<ParseIntError> for OutputError {
    fn from(err: ParseIntError) -> OutputError {
        OutputError::IntParse(err)
    }
}

impl Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputError::Empty => write!(f, "EmptyOutputError"),
            OutputError::Regex => write!(f, "RegexError"),
            OutputError::IntParse(parse_int_error) => {
                write!(f, "IntParseError: {}", parse_int_error)
            }
        }
    }
}

impl Error for OutputError {}

impl Output {
    pub fn try_parse(output: &str) -> Result<Output> {
        let last = output.lines().last().ok_or(OutputError::Empty)?;
        let re = Regex::new(
            r"\s*#SUCCESS:\s*(?P<success_n>\d+)\s*[|]\s#FAILURE:\s*(?P<failure_n>\d+)\s*",
        )
        .unwrap();
        let caps = re.captures(&last).ok_or(OutputError::Regex)?;
        let success_n: u16 = caps["success_n"].parse()?;
        let failure_n: u16 = caps["failure_n"].parse()?;
        Ok(Output {
            success_n,
            failure_n,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        assert_eq!(Err(OutputError::Empty), Output::try_parse(""));
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(
            Err(OutputError::Regex),
            Output::try_parse("#SUCCESS 10 | #FAILURE 0")
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            Ok(Output {
                success_n: 10,
                failure_n: 0
            }),
            Output::try_parse("foo\nbar\n#SUCCESS: 10 | #FAILURE: 0")
        );
    }
}
