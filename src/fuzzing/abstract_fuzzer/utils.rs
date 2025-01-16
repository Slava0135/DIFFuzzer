use crate::abstract_fs::trace::Trace;
use anyhow::Context;
use std::fs::read_to_string;
use std::path::Path;
use std::{fs, io};

pub fn setup_dir(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path).unwrap_or(());
    fs::create_dir(path)
}

pub fn parse_trace(path: &Path) -> anyhow::Result<Trace> {
    let trace = read_to_string(&path)
        .with_context(|| format!("failed to read trace at '{}'", path.display()))?;
    anyhow::Ok(Trace::try_parse(trace).with_context(|| format!("failed to parse trace"))?)
}
