/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Display;

use dash::FileDiff;
use dash::FileDiff::FileIsDifferent;
use dash::FileDiff::OnlyOneExists;

use crate::{
    abstract_fs::trace::{Trace, TraceDiff, TraceRow},
    markdown::{Language, Markdown},
};

pub struct Reason {
    pub md: Markdown,
}

impl Reason {
    pub fn new() -> Self {
        Self {
            md: Markdown::new("Reason".to_owned()),
        }
    }
    pub fn add_trace_rows(&mut self, rows: &[TraceRow]) {
        self.md.codeblock(
            Language::of("csv"),
            format!(
                "{}\n{}",
                Trace::header(),
                rows.iter()
                    .fold(String::new(), |acc, row| acc + &row.source() + "\n")
            )
            .trim()
            .to_owned(),
        );
    }
    pub fn add_trace_diff(&mut self, diff: &[TraceDiff]) {
        for diff in diff {
            match diff {
                TraceDiff::TraceRowIsDifferent { fst, snd } => self.md.codeblock(
                    Language::of("csv"),
                    format!("{}\n{}\n{}", Trace::header(), fst.source(), snd.source()),
                ),
                TraceDiff::DifferentLength => self
                    .md
                    .paragraph("Traces have different lengths".to_owned()),
            }
        }
    }
    pub fn add_dash_diff(&mut self, diff: &[FileDiff]) {
        for diff in diff {
            match diff {
                FileIsDifferent { fst, snd } => {
                    self.md.paragraph("File with different hash:".to_owned());
                    self.md
                        .codeblock(Language::of("json"), format!("{}\n{}", fst, snd));
                }
                OnlyOneExists(f) => {
                    self.md.paragraph("File exists only in one FS:".to_owned());
                    self.md.codeblock(Language::of("json"), format!("{}", f));
                }
            };
        }
    }
}

impl Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.md)
    }
}
