//! Based on POSIX.1-2024

#![allow(dead_code)]

use std::{collections::VecDeque, fmt::Display, vec};

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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
pub struct FileIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirIndex(usize);

#[derive(Debug)]
pub struct FileDescriptor(usize);

#[derive(Debug, Clone)]
pub struct File {
    pub name: Name,
    pub parent: DirIndex,
}

#[derive(Debug, Clone)]
pub struct Dir {
    pub name: Name,
    pub parent: Option<DirIndex>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    FILE(FileIndex),
    DIR(DirIndex),
}

#[derive(Debug, PartialEq)]
pub enum Operation {
    MKDIR { path: PathName, mode: Mode },
    CREATE { path: PathName, mode: Mode },
    REMOVE { path: PathName },
}

pub type Workload = Vec<Operation>;

pub struct AbstractExecutor {
    dirs: Vec<Dir>,
    files: Vec<File>,

    pub recording: Workload,
}

impl AbstractExecutor {
    pub fn new() -> Self {
        AbstractExecutor {
            dirs: vec![Dir {
                name: String::new(),
                parent: None,
                children: vec![],
            }],
            files: vec![],
            recording: vec![],
        }
    }

    pub fn remove(&mut self, node: &Node) {
        match node {
            Node::DIR(to_remove) => {
                if *to_remove == AbstractExecutor::root_index() {
                    panic!("removing root is prohibited")
                }
                self.recording.push(Operation::REMOVE {
                    path: self.resolve_path(node),
                });
                let dir = self.dir(&to_remove).clone();
                let parent = self.dir_mut(&dir.parent.unwrap());
                parent.children.retain(|n| match n {
                    Node::FILE(_) => true,
                    Node::DIR(idx) => idx != to_remove,
                });
            }
            Node::FILE(to_remove) => {
                self.recording.push(Operation::REMOVE {
                    path: self.resolve_path(node),
                });
                let file = self.file(&to_remove).clone();
                let parent = self.dir_mut(&file.parent);
                parent.children.retain(|n| match n {
                    Node::FILE(idx) => idx != to_remove,
                    Node::DIR(_) => true,
                });
            }
        }
    }

    pub fn mkdir(&mut self, parent: &DirIndex, name: Name, mode: Mode) -> DirIndex {
        if self.name_exists(&parent, &name) {
            panic!("parent directory already has a file with this name")
        }
        let dir = Dir {
            name: name,
            parent: Some(parent.clone()),
            children: vec![],
        };
        let dir_idx = DirIndex(self.dirs.len());
        self.dirs.push(dir);
        self.dir_mut(&parent).children.push(Node::DIR(dir_idx));
        self.recording.push(Operation::MKDIR {
            path: self.resolve_path(&Node::DIR(dir_idx)),
            mode: mode,
        });
        dir_idx
    }

    pub fn create(&mut self, parent: &DirIndex, name: Name, mode: Mode) -> FileIndex {
        if self.name_exists(&parent, &name) {
            panic!("parent directory already has a file with this name")
        }
        let file = File {
            name: name,
            parent: parent.clone(),
        };
        let file_idx = FileIndex(self.files.len());
        self.files.push(file);
        self.dir_mut(&parent).children.push(Node::FILE(file_idx));
        self.recording.push(Operation::CREATE {
            path: self.resolve_path(&Node::FILE(file_idx)),
            mode: mode,
        });
        file_idx
    }

    fn name_exists(&self, idx: &DirIndex, name: &Name) -> bool {
        self.dir(idx).children.iter().any(|node| match node {
            Node::DIR(idx) => &self.dir(idx).name == name,
            Node::FILE(idx) => &self.file(idx).name == name,
        })
    }

    fn dir(&self, idx: &DirIndex) -> &Dir {
        self.dirs.get(idx.0).unwrap()
    }

    fn dir_mut(&mut self, idx: &DirIndex) -> &mut Dir {
        self.dirs.get_mut(idx.0).unwrap()
    }

    fn file(&self, idx: &FileIndex) -> &File {
        self.files.get(idx.0).unwrap()
    }

    fn file_mut(&mut self, idx: &FileIndex) -> &mut File {
        self.files.get_mut(idx.0).unwrap()
    }

    fn root_mut(&mut self) -> &mut Dir {
        self.dirs.get_mut(0).unwrap()
    }

    fn root(&self) -> &Dir {
        self.dirs.get(0).unwrap()
    }

    pub fn root_index() -> DirIndex {
        DirIndex(0)
    }

    pub fn resolve_path(&self, node: &Node) -> PathName {
        let mut segments: Vec<String> = vec![];
        let mut next = node.clone();
        loop {
            match next {
                Node::DIR(idx) => {
                    let dir = self.dir(&idx);
                    match dir.parent {
                        Some(parent) => {
                            segments.push(dir.name.clone());
                            next = Node::DIR(parent.clone());
                        }
                        None => break,
                    }
                }
                Node::FILE(idx) => {
                    let file = self.file(&idx);
                    segments.push(file.name.clone());
                    next = Node::DIR(file.parent.clone()).clone();
                }
            }
        }
        segments.reverse();
        String::from("/") + segments.join("/").as_str()
    }

    pub fn alive(&self) -> Vec<Node> {
        let root = AbstractExecutor::root_index();
        let mut visited = vec![];
        let mut queue = VecDeque::new();
        queue.push_back(&root);
        visited.push(Node::DIR(root));
        while !queue.is_empty() {
            let next = queue.pop_front().unwrap();
            let dir = self.dir(&next);
            for node in dir.children.iter() {
                match node {
                    Node::DIR(idx) => {
                        queue.push_back(idx);
                        visited.push(Node::DIR(idx.clone()));
                    }
                    Node::FILE(idx) => {
                        visited.push(Node::FILE(idx.clone()));
                    }
                }
            }
        }
        visited
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_root() {
        let exec = AbstractExecutor::new();
        assert_eq!("", exec.root().name);
        assert_eq!(
            vec![Node::DIR(AbstractExecutor::root_index())],
            exec.alive()
        )
    }

    #[test]
    #[should_panic]
    fn test_remove_root() {
        let mut exec = AbstractExecutor::new();
        exec.remove(&Node::DIR(AbstractExecutor::root_index()));
    }

    #[test]
    fn test_mkdir() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.mkdir(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
        match exec.root().children[0] {
            Node::DIR(idx) => {
                assert_eq!("foobar", exec.dir(&idx).name)
            }
            _ => {
                assert!(false, "not a dir")
            }
        }
        assert_eq!(
            vec![Operation::MKDIR {
                path: String::from("/foobar"),
                mode: vec![],
            }],
            exec.recording
        );
        assert_eq!(
            vec![Node::DIR(AbstractExecutor::root_index()), Node::DIR(foo)],
            exec.alive()
        )
    }

    #[test]
    #[should_panic]
    fn test_mkdir_same_name() {
        let mut exec = AbstractExecutor::new();
        exec.mkdir(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
        exec.mkdir(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
    }

    #[test]
    fn test_create() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.create(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
        match exec.root().children[0] {
            Node::FILE(idx) => {
                assert_eq!("foobar", exec.file(&idx).name)
            }
            _ => {
                assert!(false, "not a file")
            }
        }
        assert_eq!(
            vec![Node::DIR(AbstractExecutor::root_index()), Node::FILE(foo)],
            exec.alive()
        );
        assert_eq!(
            vec![Operation::CREATE {
                path: String::from("/foobar"),
                mode: vec![],
            }],
            exec.recording
        )
    }

    #[test]
    #[should_panic]
    fn test_create_same_name() {
        let mut exec = AbstractExecutor::new();
        exec.create(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
        exec.create(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
    }

    #[test]
    fn test_remove_file() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.create(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
        let boo = exec.create(&AbstractExecutor::root_index(), String::from("boo"), vec![]);
        let mut expected = vec![
            Node::DIR(AbstractExecutor::root_index()),
            Node::FILE(foo),
            Node::FILE(boo),
        ];
        let mut actual = exec.alive();
        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);

        exec.remove(&Node::FILE(foo));

        assert_eq!(1, exec.root().children.len());
        match exec.root().children[0] {
            Node::FILE(idx) => {
                assert_eq!("boo", exec.file(&idx).name)
            }
            _ => {
                assert!(false, "not a file")
            }
        }
        let mut expected = vec![Node::DIR(AbstractExecutor::root_index()), Node::FILE(boo)];
        let mut actual = exec.alive();
        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);
        assert_eq!(
            vec![
                Operation::CREATE {
                    path: String::from("/foobar"),
                    mode: vec![],
                },
                Operation::CREATE {
                    path: String::from("/boo"),
                    mode: vec![],
                },
                Operation::REMOVE {
                    path: String::from("/foobar")
                }
            ],
            exec.recording
        )
    }

    #[test]
    fn test_remove_dir() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.mkdir(
            &AbstractExecutor::root_index(),
            String::from("foobar"),
            vec![],
        );
        let boo = exec.mkdir(&AbstractExecutor::root_index(), String::from("boo"), vec![]);
        let mut expected = vec![
            Node::DIR(AbstractExecutor::root_index()),
            Node::DIR(foo),
            Node::DIR(boo),
        ];
        let mut actual = exec.alive();
        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);

        exec.remove(&Node::DIR(foo));

        assert_eq!(1, exec.root().children.len());
        match exec.root().children[0] {
            Node::DIR(idx) => {
                assert_eq!("boo", exec.dir(&idx).name)
            }
            _ => {
                assert!(false, "not a dir")
            }
        }
        let mut expected = vec![Node::DIR(AbstractExecutor::root_index()), Node::DIR(boo)];
        let mut actual = exec.alive();
        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);
        assert_eq!(
            vec![
                Operation::MKDIR {
                    path: String::from("/foobar"),
                    mode: vec![],
                },
                Operation::MKDIR {
                    path: String::from("/boo"),
                    mode: vec![],
                },
                Operation::REMOVE {
                    path: String::from("/foobar")
                }
            ],
            exec.recording
        )
    }
}
