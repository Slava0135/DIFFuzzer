use serde::{Deserialize, Serialize};

use super::{flags::Mode, node::FileDescriptor, pathname::PathName};

#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize)]
pub enum Operation {
    MKDIR {
        path: PathName,
        mode: Mode,
    },
    CREATE {
        path: PathName,
        mode: Mode,
    },
    REMOVE {
        path: PathName,
    },
    HARDLINK {
        old_path: PathName,
        new_path: PathName,
    },
    RENAME {
        old_path: PathName,
        new_path: PathName,
    },
    OPEN {
        path: PathName,
        des: FileDescriptor,
    },
    CLOSE {
        des: FileDescriptor
    },
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Copy)]
pub enum OperationKind {
    MKDIR,
    CREATE,
    REMOVE,
    HARDLINK,
    RENAME,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OperationWeights {
    pub weights: Vec<(OperationKind, u32)>,
}

impl OperationWeights {
    pub fn new(weights: Vec<(OperationKind, u32)>) -> Self {
        Self { weights }
    }

    pub fn uniform() -> Self {
        Self {
            weights: vec![
                (OperationKind::CREATE, 100),
                (OperationKind::MKDIR, 100),
                (OperationKind::REMOVE, 100),
                (OperationKind::HARDLINK, 100),
                (OperationKind::RENAME, 100),
            ],
        }
    }
}
