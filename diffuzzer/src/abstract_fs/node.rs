/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};

use super::{
    content::Content,
    pathname::{Name, PathName},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymlinkIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FileDescriptorIndex(pub usize);

impl Display for FileDescriptorIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub descriptor: Option<FileDescriptorIndex>,
    pub content: Content,
}

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub file: FileIndex,
    pub offset: u64,
}

#[derive(Debug, Clone)]
pub struct Dir {
    pub children: HashMap<Name, Node>,
}

pub struct Symlink {
    pub target: PathName,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    File(FileIndex),
    Dir(DirIndex),
    Symlink(SymlinkIndex),
}
