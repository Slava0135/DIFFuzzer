use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathName(String);

impl Display for PathName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for PathName {
    fn from(value: &str) -> Self {
        PathName(value.to_owned())
    }
}

impl From<String> for PathName {
    fn from(value: String) -> Self {
        PathName(value)
    }
}

impl PathName {
    pub fn split(&self) -> (PathName, Name) {
        let split_at = self.0.rfind('/').unwrap();
        let (parent, name) = (&self.0[..split_at], &self.0[split_at + 1..]);
        if parent.is_empty() {
            ("/".into(), name.to_owned())
        } else {
            (parent.into(), name.to_owned())
        }
    }

    pub fn segments(&self) -> Vec<&str> {
        self.0.split("/").filter(|s| !s.is_empty()).collect()
    }

    pub fn join(&self, name: Name) -> PathName {
        if self.is_root() {
            format!("/{}", name).into()
        } else {
            format!("{}/{}", self.0, name).into()
        }
    }

    pub fn is_valid(&self) -> bool {
        !(self.0.is_empty()
            || !self.0.starts_with('/')
            || (!self.is_root() && self.0.ends_with('/')))
    }

    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }
}

pub type Name = String;
