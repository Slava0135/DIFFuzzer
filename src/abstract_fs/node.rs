use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
};

use serde::{Deserialize, Serialize};

use super::pathname::Name;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FileDescriptor(pub usize);

impl Display for FileDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub descriptor: Option<FileDescriptor>,
    pub content: Content,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceSlice {
    pub from: u64,
    pub to: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Content {
    slices: VecDeque<SourceSlice>,
}

impl Content {
    pub fn new() -> Self {
        Self {
            slices: VecDeque::new(),
        }
    }
    pub fn slices(&self) -> Vec<SourceSlice> {
        self.slices.iter().map(|s| s.to_owned()).collect()
    }
    pub fn write(&mut self, src_offset: u64, size: u64) {
        let old_sise = self.size();
        if size > 0 {
            let mut truncate_size = size;
            for slice in self.slices.iter_mut() {
                let can_truncate = slice.to - slice.from + 1;
                if can_truncate > truncate_size {
                    slice.from += truncate_size;
                    break;
                }
                slice.from = slice.to;
                truncate_size -= can_truncate;
            }
            self.slices.retain(|s| s.from != s.to);
            self.slices.push_front(SourceSlice {
                from: src_offset,
                to: src_offset + size - 1,
            });
        }
        let new_size = self.size();
        if size < old_sise {
            assert!(
                new_size == old_sise,
                "new_size = {}, old_size = {}:\n{:?}",
                new_size,
                old_sise,
                self.slices
            )
        } else {
            assert!(
                new_size == size,
                "new_size = {}, size = {}:\n{:?}",
                new_size,
                size,
                self.slices
            )
        }
        for s in self.slices.iter() {
            assert!(
                s.from < s.to,
                "from = {}, to = {}:\n{:?}",
                s.from,
                s.to,
                self.slices
            );
        }
    }
    pub fn size(&self) -> u64 {
        self.slices
            .iter()
            .fold(0, |acc: u64, x| acc + x.to - x.from + 1)
    }
}

#[derive(Debug, Clone)]
pub struct Dir {
    pub children: HashMap<Name, Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    FILE(FileIndex),
    DIR(DirIndex),
}
