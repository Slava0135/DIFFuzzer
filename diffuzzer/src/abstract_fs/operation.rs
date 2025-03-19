/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use serde::{Deserialize, Serialize};

use super::{flags::Mode, node::FileDescriptorIndex, pathname::PathName};

#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Operation {
    MkDir {
        path: PathName,
        mode: Mode,
    },
    Create {
        path: PathName,
        mode: Mode,
    },
    Remove {
        path: PathName,
    },
    Hardlink {
        old_path: PathName,
        new_path: PathName,
    },
    Rename {
        old_path: PathName,
        new_path: PathName,
    },
    Open {
        path: PathName,
        des: FileDescriptorIndex,
    },
    Close {
        des: FileDescriptorIndex,
    },
    Read {
        des: FileDescriptorIndex,
        size: u64,
    },
    Write {
        des: FileDescriptorIndex,
        src_offset: u64,
        size: u64,
    },
    FSync {
        des: FileDescriptorIndex,
    },
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum OperationKind {
    MkDir,
    Create,
    Remove,
    Hardlink,
    Rename,
    Open,
    Close,
    Read,
    Write,
    FSync,
}

impl From<&Operation> for OperationKind {
    fn from(value: &Operation) -> Self {
        match value {
            Operation::MkDir { .. } => Self::MkDir,
            Operation::Create { .. } => Self::Create,
            Operation::Remove { .. } => Self::Remove,
            Operation::Hardlink { .. } => Self::Hardlink,
            Operation::Rename { .. } => Self::Rename,
            Operation::Open { .. } => Self::Open,
            Operation::Close { .. } => Self::Close,
            Operation::Read { .. } => Self::Read,
            Operation::Write { .. } => Self::Write,
            Operation::FSync { .. } => Self::FSync,
        }
    }
}

impl From<Operation> for OperationKind {
    fn from(value: Operation) -> Self {
        match value {
            Operation::MkDir { .. } => Self::MkDir,
            Operation::Create { .. } => Self::Create,
            Operation::Remove { .. } => Self::Remove,
            Operation::Hardlink { .. } => Self::Hardlink,
            Operation::Rename { .. } => Self::Rename,
            Operation::Open { .. } => Self::Open,
            Operation::Close { .. } => Self::Close,
            Operation::Read { .. } => Self::Read,
            Operation::Write { .. } => Self::Write,
            Operation::FSync { .. } => Self::FSync,
        }
    }
}

/// Weights determine the likelihood of operation to be picked.
/// Weight is only considered if operation can be applied.
#[derive(Serialize, Deserialize, Clone)]
pub struct OperationWeights {
    pub weights: Vec<(OperationKind, u32)>,
}

#[allow(dead_code)]
impl OperationWeights {
    pub fn new(weights: Vec<(OperationKind, u32)>) -> Self {
        Self { weights }
    }

    pub fn uniform() -> Self {
        Self {
            weights: vec![
                (OperationKind::Create, 100),
                (OperationKind::MkDir, 100),
                (OperationKind::Remove, 100),
                (OperationKind::Hardlink, 100),
                (OperationKind::Rename, 100),
                (OperationKind::Open, 100),
                (OperationKind::Close, 100),
                (OperationKind::Read, 100),
                (OperationKind::Write, 100),
                (OperationKind::FSync, 100),
            ],
        }
    }
}
