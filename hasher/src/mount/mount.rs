use std::fmt::Display;

use regex::RegexSet;

pub trait FileSystemMount: Display {
    fn get_internal_dirs(&self) -> RegexSet {
        RegexSet::new::<_, &str>([]).unwrap()
    }
}
