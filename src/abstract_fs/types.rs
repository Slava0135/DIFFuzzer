//! Based on POSIX.1-2024

#![allow(dead_code)]

use std::{collections::HashMap, fmt::Display, vec};

use libafl::SerdeAny;
use serde::{Deserialize, Serialize};

/// Flags for `open(path, flags, mode)` syscall.
///
/// Applications *shall* specify __exactly one__ of the __first 5__ values.
#[derive(Debug, PartialEq, Eq, Hash)]
#[allow(nonstandard_style)]
pub enum OpenFlag {
    /// Open for execute only (non-directory files).
    /// If path names a directory and `O_EXEC` is not the same value as `O_SEARCH`, `open()` shall fail.
    O_EXEC,
    /// Open for reading only.
    O_RDONLY,
    /// Open for reading and writing.
    /// If path names a `FIFO`, and the implementation does not support opening a `FIFO` for simultaneous read and write, then `open()` shall fail.
    O_RDWR,
    /// Open directory for search only. If path names a non-directory file and `O_SEARCH` is not the same value as `O_EXEC`, `open()` shall fail.
    O_SEARCH,
    /// Open for writing only.
    O_WRONLY,

    /// If set, the file offset shall be set to the end of the file prior to each write.
    O_APPEND,
    /// If set, the `FD_CLOEXEC` flag for the new file descriptor shall be set.
    O_CLOEXEC,
    /// If set, the `FD_CLOFORK` flag for the new file descriptor shall be set.
    O_CLOFORK,
    /// If the file exists, this flag has no effect except as noted under `O_EXCL` below.
    /// Otherwise, if `O_DIRECTORY` is not set the file shall be created as a regular file.
    ///
    /// __LINUX__: The `mode` argument __must__ be supplied if `O_CREAT` or `O_TMPFILE` is specified in flags.
    O_CREAT,
    /// If path resolves to a non-directory file, fail and set errno to `ENOTDIR`.
    O_DIRECTORY,
    /// Write I/O operations on the file descriptor shall complete as defined by synchronized I/O data integrity completion.
    O_DSYNC,
    /// If `O_CREAT` and `O_EXCL` are set, `open()` shall fail if the file exists.
    /// If `O_EXCL` and `O_CREAT` are set, and path names a symbolic link, `open()` shall fail and set errno to `EEXIST`, regardless of the contents of the symbolic link.
    /// If `O_EXCL` is set and `O_CREAT` is not set, the result is undefined.
    O_EXCL,
    /// If set and path identifies a terminal device, `open()` shall not cause the terminal device to become the controlling terminal for the process.
    /// If path does not identify a terminal device, O_NOCTTY shall be ignored.
    O_NOCTTY,
    /// If path names a symbolic link, fail and set errno to `ELOOP`.
    O_NOFOLLOW,
    /// When opening a FIFO with `O_RDONLY` or `O_WRONLY` set:
    ///
    /// - If `O_NONBLOCK` is set, an `open()` for reading-only shall return without delay. An `open()` for writing-only shall return an error if no process currently has the file open for reading.
    /// - If `O_NONBLOCK` is clear, an `open()` for reading-only shall block the calling thread until a thread opens the file for writing. An `open()` for writing-only shall block the calling thread until a thread opens the file for reading.
    ///
    /// When opening a block special or character special file that supports non-blocking opens:
    ///
    /// - If `O_NONBLOCK` is set, the `open()` function shall return without blocking for the device to be ready or available. Subsequent behavior of the device is device-specific.
    /// - If `O_NONBLOCK` is clear, the `open()` function shall block the calling thread until the device is ready or available before returning.
    ///
    O_NONBLOCK,
    /// Read I/O operations on the file descriptor shall complete at the same level of integrity as specified by the `O_DSYNC` and `O_SYNC` flags.
    /// If both `O_DSYNC` and `O_RSYNC` are set in oflag, all I/O operations on the file descriptor shall complete as defined by synchronized I/O data integrity completion.
    /// If both `O_SYNC` and `O_RSYNC` are set in flags, all I/O operations on the file descriptor shall complete as defined by synchronized I/O file integrity completion.
    O_RSYNC,
    ///  Write I/O operations on the file descriptor shall complete as defined by synchronized I/O file integrity completion.
    O_SYNC,
    /// If the file exists and is a regular file, and the file is successfully opened `O_RDWR` or `O_WRONLY`, its length shall be truncated to 0, and the mode and owner shall be unchanged.
    /// It shall have no effect on `FIFO` special files or terminal device files.
    /// Its effect on other file types is implementation-defined.
    /// The result of using `O_TRUNC` without either `O_RDWR` or `O_WRONLY` is undefined.
    O_TRUNC,

    O_TTY_INIT,
}
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, SerdeAny)]
#[allow(nonstandard_style)]
pub enum ModeFlag {
    /// Read, write, execute/search by owner.
    S_IRWXU = 0o700,
    /// Read permission, owner.
    S_IRUSR = 0o400,
    /// Write permission, owner.
    S_IWUSR = 0o200,
    /// Execute/search permission, owner.
    S_IXUSR = 0o100,
    /// Read, write, execute/search by group.
    S_IRWXG = 0o70,
    /// Read permission, group.
    S_IRGRP = 0o40,
    /// Write permission, group.
    S_IWGRP = 0o20,
    /// Execute/search permission, group.
    S_IXGRP = 0o10,
    /// Read, write, execute/search by others.
    S_IRWXO = 0o7,
    /// Read permission, others.
    S_IROTH = 0o4,
    /// Write permission, others.
    S_IWOTH = 0o2,
    /// Execute/search permission, others.
    S_IXOTH = 0o1,

    /// Set-user-ID on execution.
    S_ISUID = 0o4000,
    /// Set-group-ID on execution.
    S_ISGID = 0o2000,
    /// On directories, restricted deletion flag.
    S_ISVTX = 0o1000,
}

impl Display for ModeFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeFlag::S_IRWXU => write!(f, "S_IRWXU"),
            ModeFlag::S_IRUSR => write!(f, "S_IRUSR"),
            ModeFlag::S_IWUSR => write!(f, "S_IWUSR"),
            ModeFlag::S_IXUSR => write!(f, "S_IXUSR"),
            ModeFlag::S_IRWXG => write!(f, "S_IRWXG"),
            ModeFlag::S_IRGRP => write!(f, "S_IRGRP"),
            ModeFlag::S_IWGRP => write!(f, "S_IWGRP"),
            ModeFlag::S_IXGRP => write!(f, "S_IXGRP"),
            ModeFlag::S_IRWXO => write!(f, "S_IRWXO"),
            ModeFlag::S_IROTH => write!(f, "S_IROTH"),
            ModeFlag::S_IWOTH => write!(f, "S_IWOTH"),
            ModeFlag::S_IXOTH => write!(f, "S_IXOTH"),
            ModeFlag::S_ISUID => write!(f, "S_ISUID"),
            ModeFlag::S_ISGID => write!(f, "S_ISGID"),
            ModeFlag::S_ISVTX => write!(f, "S_ISVTX"),
        }
    }
}

pub type Mode = Vec<ModeFlag>;

pub type PathName = String;
pub type Name = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileIndex(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirIndex(pub usize);

#[derive(Debug)]
pub struct FileDescriptor(usize);

#[derive(Debug, Clone)]
pub struct File {
    pub parent: DirIndex,
}

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

#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize, SerdeAny)]
pub enum Operation {
    MKDIR { path: PathName, mode: Mode },
    CREATE { path: PathName, mode: Mode },
    REMOVE { path: PathName },
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Copy)]
pub enum OperationKind {
    MKDIR,
    CREATE,
    REMOVE,
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
            ],
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum MutationKind {
    INSERT,
    REMOVE,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MutationWeights {
    pub weights: Vec<(MutationKind, u32)>,
}

impl MutationWeights {
    pub fn new(weights: Vec<(MutationKind, u32)>) -> Self {
        Self { weights }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Serialize, Deserialize, SerdeAny)]
pub struct Workload {
    pub ops: Vec<Operation>,
}

impl Workload {
    pub fn new() -> Workload {
        Workload { ops: vec![] }
    }
    pub fn push(&mut self, op: Operation) {
        self.ops.push(op);
    }
}

pub struct AbstractExecutor {
    pub dirs: Vec<Dir>,
    pub files: Vec<File>,
    pub nodes_created: usize,
    pub recording: Workload,
}
