use std::fmt::Display;

use regex::RegexSet;

use super::mount::FileSystemMount;

pub struct F2FS;

impl Display for F2FS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "F2FS")
    }
}

impl FileSystemMount for F2FS {
    fn get_internal_dirs(&self) -> RegexSet {
        RegexSet::new([r"^/?lost\+found($|/)"]).unwrap()
    }
}

impl F2FS {
    pub const fn new() -> Self {
        Self {}
    }
}
