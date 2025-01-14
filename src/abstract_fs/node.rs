use std::collections::HashMap;

use super::pathname::Name;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirIndex(pub usize);

#[derive(Debug)]
pub struct FileDescriptor(usize);

#[derive(Debug, Clone)]
pub struct File;

#[derive(Debug, Clone)]
pub struct Dir {
    pub parent: Option<DirIndex>,
    pub children: HashMap<Name, Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    FILE(FileIndex),
    DIR(DirIndex),
}
