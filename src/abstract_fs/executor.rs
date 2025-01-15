use std::collections::{HashMap, VecDeque};

use thiserror::Error;

use super::{
    flags::Mode,
    node::{Dir, DirIndex, File, FileDescriptor, FileIndex, Node},
    operation::Operation,
    pathname::{Name, PathName},
    workload::Workload,
};

type Result<T> = std::result::Result<T, ExecutorError>;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ExecutorError {
    #[error("'{0}' is not a file")]
    NotAFile(PathName),
    #[error("'{0}' is not a dir")]
    NotADir(PathName),
    #[error("node at '{0}' already exists")]
    NameAlreadyExists(PathName),
    #[error("removing root is forbidden")]
    RootRemovalForbidden,
    #[error("node at path '{0}' not found")]
    NotFound(PathName),
    #[error("invalid path '{0}'")]
    InvalidPath(PathName),
    #[error("directory '{0}' is not empty")]
    DirNotEmpty(PathName),
    #[error("bad descriptor '{0}' ({1} created)")]
    BadDescriptor(FileDescriptor, usize),
    #[error("descriptor '{0}' was already closed")]
    DescriptorWasClosed(FileDescriptor),
}

pub struct AbstractExecutor {
    pub dirs: Vec<Dir>,
    pub files: Vec<File>,

    pub descriptors: Vec<FileIndex>,

    pub recording: Workload,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AliveNodes {
    pub dirs: Vec<PathName>,
    pub files: Vec<PathName>,
}

impl AbstractExecutor {
    pub fn new() -> Self {
        AbstractExecutor {
            dirs: vec![Dir {
                children: HashMap::new(),
            }],
            files: vec![],
            descriptors: vec![],
            recording: Workload::new(),
        }
    }

    pub fn remove(&mut self, path: PathName) -> Result<()> {
        if path.is_root() {
            return Err(ExecutorError::RootRemovalForbidden);
        }
        let (parent_path, name) = path.split();
        let parent_idx = self.resolve_dir(parent_path.to_owned())?;
        let parent = self.dir_mut(&parent_idx);
        parent.children.remove(&name);
        self.recording
            .push(Operation::REMOVE { path: path.clone() });
        Ok(())
    }

    pub fn mkdir(&mut self, path: PathName, mode: Mode) -> Result<DirIndex> {
        let (parent_path, name) = path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(ExecutorError::NameAlreadyExists(path));
        }
        let dir = Dir {
            children: HashMap::new(),
        };
        let dir_idx = DirIndex(self.dirs.len());
        self.dirs.push(dir);
        self.dir_mut(&parent)
            .children
            .insert(name, Node::DIR(dir_idx));
        self.recording.push(Operation::MKDIR { path, mode });
        Ok(dir_idx)
    }

    pub fn create(&mut self, path: PathName, mode: Mode) -> Result<FileIndex> {
        let (parent_path, name) = path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(ExecutorError::NameAlreadyExists(path));
        }
        let file = File { is_open: false };
        let file_idx = FileIndex(self.files.len());
        self.files.push(file);
        self.dir_mut(&parent)
            .children
            .insert(name.clone(), Node::FILE(file_idx));
        self.recording.push(Operation::CREATE { path, mode });
        Ok(file_idx)
    }

    pub fn hardlink(&mut self, old_path: PathName, new_path: PathName) -> Result<FileIndex> {
        let old_file = self.resolve_file(old_path.clone())?;
        let (parent_path, name) = new_path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(ExecutorError::NameAlreadyExists(new_path));
        }
        let parent_dir = self.dir_mut(&parent);
        parent_dir
            .children
            .insert(name.clone(), Node::FILE(old_file.to_owned()));
        self.recording
            .push(Operation::HARDLINK { old_path, new_path });
        Ok(old_file.to_owned())
    }

    pub fn rename(&mut self, old_path: PathName, new_path: PathName) -> Result<Node> {
        if let Ok(dir_idx) = self.resolve_dir(new_path.clone()) {
            if !self.dir(&dir_idx).children.is_empty() {
                return Err(ExecutorError::DirNotEmpty(new_path));
            }
        }
        let node = self.resolve_node(old_path.clone())?;

        let (parent_path, name) = new_path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        let parent_dir = self.dir_mut(&parent);
        parent_dir.children.insert(name.clone(), node.clone());

        let (parent_path, name) = old_path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        let parent_dir = self.dir_mut(&parent);
        parent_dir.children.remove(&name);

        self.recording
            .push(Operation::RENAME { old_path, new_path });
        Ok(node)
    }

    pub fn open(&mut self, path: PathName) -> Result<FileDescriptor> {
        let file_idx = self.resolve_file(path.clone())?;
        let file = self.file_mut(&file_idx);
        file.is_open = true;
        let des = FileDescriptor(self.descriptors.len());
        self.descriptors.push(file_idx);
        self.recording.push(Operation::OPEN { path });
        Ok(des)
    }

    pub fn close(&mut self, des: FileDescriptor) -> Result<()> {
        let file_idx = self
            .descriptors
            .get(des.0)
            .ok_or(ExecutorError::BadDescriptor(des, self.descriptors.len()))?;
        let file = self.file_mut(&file_idx.clone());
        if !file.is_open {
            return Err(ExecutorError::DescriptorWasClosed(des));
        }
        file.is_open = false;
        self.recording.push(Operation::CLOSE { des });
        Ok(())
    }

    pub fn replay(&mut self, workload: &Workload) -> Result<()> {
        for op in &workload.ops {
            match op {
                Operation::MKDIR { path, mode } => {
                    self.mkdir(path.clone(), mode.clone())?;
                }
                Operation::CREATE { path, mode } => {
                    self.create(path.clone(), mode.clone())?;
                }
                Operation::REMOVE { path } => self.remove(path.clone())?,
                Operation::HARDLINK { old_path, new_path } => {
                    self.hardlink(old_path.clone(), new_path.clone())?;
                }
                Operation::RENAME { old_path, new_path } => {
                    self.rename(old_path.clone(), new_path.clone())?;
                }
                Operation::OPEN { path } => {
                    self.open(path.clone())?;
                }
                Operation::CLOSE { des } => {
                    self.close(des.clone())?;
                }
            };
        }
        Ok(())
    }

    fn name_exists(&self, idx: &DirIndex, name: &Name) -> bool {
        self.dir(idx).children.contains_key(name)
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

    pub fn resolve_node(&self, path: PathName) -> Result<Node> {
        if !path.is_valid() {
            return Err(ExecutorError::InvalidPath(path));
        }
        let segments: Vec<&str> = path.segments();
        let mut last = Node::DIR(AbstractExecutor::root_index());
        let mut path = String::new();
        for segment in &segments {
            path.push_str("/");
            path.push_str(segment);
            let dir = match last {
                Node::DIR(dir_index) => self.dir(&dir_index),
                _ => return Err(ExecutorError::NotADir(path.into())),
            };
            last = dir
                .children
                .get(segment.to_owned())
                .ok_or(ExecutorError::NotFound(path.clone().into()))?
                .clone();
        }
        Ok(last)
    }

    pub fn resolve_file(&self, path: PathName) -> Result<FileIndex> {
        match self.resolve_node(path.clone())? {
            Node::FILE(idx) => Ok(idx),
            _ => Err(ExecutorError::NotAFile(path)),
        }
    }

    pub fn resolve_dir(&self, path: PathName) -> Result<DirIndex> {
        match self.resolve_node(path.clone())? {
            Node::DIR(idx) => Ok(idx),
            _ => Err(ExecutorError::NotADir(path)),
        }
    }

    pub fn root_index() -> DirIndex {
        DirIndex(0)
    }

    pub fn alive(&self) -> AliveNodes {
        let root = AbstractExecutor::root_index();
        let mut alive = AliveNodes {
            dirs: vec![],
            files: vec![],
        };
        let mut queue: VecDeque<(PathName, &DirIndex)> = VecDeque::new();
        queue.push_back(("/".into(), &root));
        alive.dirs.push("/".into());
        while let Some((path, idx)) = queue.pop_front() {
            let dir = self.dir(idx);
            for (name, node) in dir.children.iter() {
                match node {
                    Node::DIR(idx) => {
                        let path = path.join(name.to_owned());
                        queue.push_back((path.clone(), idx));
                        alive.dirs.push(path.clone());
                    }
                    Node::FILE(_) => {
                        alive.files.push(path.join(name.to_owned()));
                    }
                }
            }
        }
        alive.dirs.sort();
        alive.files.sort();
        alive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_root() {
        let exec = AbstractExecutor::new();
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec![]
            },
            exec.alive()
        )
    }

    #[test]
    fn test_remove_root() {
        let mut exec = AbstractExecutor::new();
        assert_eq!(
            Err(ExecutorError::RootRemovalForbidden),
            exec.remove("/".into())
        );
    }

    #[test]
    fn test_mkdir() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.mkdir("/foobar".into(), vec![]).unwrap();
        assert_eq!(Node::DIR(foo), *exec.root().children.get("foobar").unwrap());
        assert_eq!(
            Workload {
                ops: vec![Operation::MKDIR {
                    path: "/foobar".into(),
                    mode: vec![],
                }],
            },
            exec.recording
        );
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/foobar".into()],
                files: vec![]
            },
            exec.alive()
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_mkdir_name_exists() {
        let mut exec = AbstractExecutor::new();
        exec.mkdir("/foobar".into(), vec![]).unwrap();
        assert_eq!(
            Err(ExecutorError::NameAlreadyExists("/foobar".into())),
            exec.mkdir("/foobar".into(), vec![])
        );
    }

    #[test]
    fn test_create() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.create("/foobar".into(), vec![]).unwrap();
        assert_eq!(
            Node::FILE(foo),
            *exec.root().children.get("foobar").unwrap()
        );
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec!["/foobar".into()]
            },
            exec.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![Operation::CREATE {
                    path: "/foobar".into(),
                    mode: vec![],
                }]
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_create_name_exists() {
        let mut exec = AbstractExecutor::new();
        exec.create("/foobar".into(), vec![]).unwrap();
        assert_eq!(
            Err(ExecutorError::NameAlreadyExists("/foobar".into())),
            exec.create("/foobar".into(), vec![])
        );
    }

    #[test]
    fn test_remove_file() {
        let mut exec = AbstractExecutor::new();
        exec.create("/foobar".into(), vec![]).unwrap();
        let boo = exec.create("/boo".into(), vec![]).unwrap();

        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec!["/boo".into(), "/foobar".into()]
            },
            exec.alive()
        );

        exec.remove("/foobar".into()).unwrap();

        assert_eq!(1, exec.root().children.len());
        assert_eq!(Node::FILE(boo), *exec.root().children.get("boo").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec!["/boo".into()]
            },
            exec.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foobar".into(),
                        mode: vec![],
                    },
                    Operation::CREATE {
                        path: "/boo".into(),
                        mode: vec![],
                    },
                    Operation::REMOVE {
                        path: "/foobar".into(),
                    }
                ],
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_hardlink() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.create("/foo".into(), vec![]).unwrap();
        let bar = exec.mkdir("/bar".into(), vec![]).unwrap();
        let boo = exec.hardlink("/foo".into(), "/bar/boo".into()).unwrap();

        assert_eq!(foo, boo);
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/bar".into()],
                files: vec!["/bar/boo".into(), "/foo".into()]
            },
            exec.alive()
        );

        let root = exec.root();
        let bar_dir = exec.dir(&bar);
        assert_eq!(2, root.children.len());
        assert_eq!(1, bar_dir.children.len());
        assert_eq!(
            root.children.get("foo").unwrap(),
            bar_dir.children.get("boo").unwrap()
        );

        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![],
                    },
                    Operation::MKDIR {
                        path: "/bar".into(),
                        mode: vec![],
                    },
                    Operation::HARDLINK {
                        old_path: "/foo".into(),
                        new_path: "/bar/boo".into(),
                    }
                ],
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_remove_hardlink() {
        let mut exec = AbstractExecutor::new();
        exec.create("/foo".into(), vec![]).unwrap();
        exec.hardlink("/foo".into(), "/bar".into()).unwrap();
        exec.remove("/bar".into()).unwrap();

        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec!["/foo".into()]
            },
            exec.alive()
        );

        assert_eq!(1, exec.root().children.len());

        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![],
                    },
                    Operation::HARDLINK {
                        old_path: "/foo".into(),
                        new_path: "/bar".into(),
                    },
                    Operation::REMOVE {
                        path: "/bar".into(),
                    }
                ],
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_remove_hardlink_dir() {
        let mut exec = AbstractExecutor::new();
        let zero = exec.create("/0".into(), vec![]).unwrap();
        exec.mkdir("/1".into(), vec![]).unwrap();
        exec.mkdir("/1/2".into(), vec![]).unwrap();
        exec.hardlink("/0".into(), "/1/2/3".into()).unwrap();
        assert_eq!(Ok(zero), exec.resolve_file("/1/2/3".into()));
        exec.remove("/1".into()).unwrap();
        assert_eq!(
            Err(ExecutorError::NotFound("/1".into())),
            exec.resolve_file("/1/2/3".into())
        );
        assert_eq!(Ok(zero), exec.resolve_file("/0".into()));
    }

    #[test]
    fn test_hardlink_name_exists() {
        let mut exec = AbstractExecutor::new();
        exec.create("/foo".into(), vec![]).unwrap();
        exec.create("/bar".into(), vec![]).unwrap();
        assert_eq!(
            Err(ExecutorError::NameAlreadyExists("/foo".into())),
            exec.hardlink("/bar".into(), "/foo".into())
        );
    }

    #[test]
    fn test_remove_dir() {
        let mut exec = AbstractExecutor::new();
        exec.mkdir("/foobar".into(), vec![]).unwrap();
        let boo = exec.mkdir("/boo".into(), vec![]).unwrap();

        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/boo".into(), "/foobar".into()],
                files: vec![]
            },
            exec.alive()
        );

        exec.remove("/foobar".into()).unwrap();

        assert_eq!(1, exec.root().children.len());
        assert_eq!(Node::DIR(boo), *exec.root().children.get("boo").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/boo".into()],
                files: vec![]
            },
            exec.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::MKDIR {
                        path: "/foobar".into(),
                        mode: vec![],
                    },
                    Operation::MKDIR {
                        path: "/boo".into(),
                        mode: vec![],
                    },
                    Operation::REMOVE {
                        path: "/foobar".into(),
                    }
                ],
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_rename_file() {
        let mut exec = AbstractExecutor::new();
        exec.create("/foo".into(), vec![]).unwrap();
        exec.rename("/foo".into(), "/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec!["/bar".into()]
            },
            exec.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::RENAME {
                        old_path: "/foo".into(),
                        new_path: "/bar".into(),
                    }
                ]
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_rename_dir() {
        let mut exec = AbstractExecutor::new();
        exec.mkdir("/foo".into(), vec![]).unwrap();
        exec.rename("/foo".into(), "/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/bar".into()],
                files: vec![]
            },
            exec.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::MKDIR {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::RENAME {
                        old_path: "/foo".into(),
                        new_path: "/bar".into(),
                    }
                ]
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_rename_dir_non_empty() {
        let mut exec = AbstractExecutor::new();
        exec.mkdir("/foo".into(), vec![]).unwrap();
        exec.mkdir("/bar".into(), vec![]).unwrap();
        exec.create("/bar/baz".into(), vec![]).unwrap();
        assert_eq!(
            Err(ExecutorError::DirNotEmpty("/bar".into())),
            exec.rename("/foo".into(), "/bar".into())
        );
        exec.remove("/bar/baz".into()).unwrap();
        exec.rename("/foo".into(), "/bar".into()).unwrap();
    }

    #[test]
    fn test_open_close_file() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.create("/foo".into(), vec![]).unwrap();
        let des = exec.open("/foo".into()).unwrap();
        let file = exec.file(&foo);
        assert_eq!(true, file.is_open);
        exec.close(des).unwrap();
        let file = exec.file(&foo);
        assert_eq!(false, file.is_open);
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::OPEN {
                        path: "/foo".into(),
                    },
                    Operation::CLOSE { des }
                ]
            },
            exec.recording
        );
        test_replay(exec.recording);
    }

    #[test]
    fn test_close_bad_descriptor() {
        let mut exec = AbstractExecutor::new();
        let des = FileDescriptor(0);
        assert_eq!(Err(ExecutorError::BadDescriptor(des, 0)), exec.close(des));
    }

    #[test]
    fn test_close_twice() {
        let mut exec = AbstractExecutor::new();
        exec.create("/foo".into(), vec![]).unwrap();
        let des = exec.open("/foo".into()).unwrap();
        exec.close(des).unwrap();
        assert_eq!(
            Err(ExecutorError::DescriptorWasClosed(des)),
            exec.close(des)
        );
    }

    #[test]
    fn test_resolve_node() {
        let mut exec = AbstractExecutor::new();
        assert_eq!(
            Node::DIR(AbstractExecutor::root_index()),
            exec.resolve_node("/".into()).unwrap()
        );
        let foo = exec.mkdir("/foo".into(), vec![]).unwrap();
        let bar = exec.mkdir("/foo/bar".into(), vec![]).unwrap();
        let boo = exec.create("/foo/bar/boo".into(), vec![]).unwrap();
        assert_eq!(
            Err(ExecutorError::InvalidPath("".into())),
            exec.resolve_node("".into())
        );
        assert_eq!(
            Err(ExecutorError::InvalidPath("foo".into())),
            exec.resolve_node("foo".into())
        );
        assert_eq!(
            Err(ExecutorError::InvalidPath("/foo/".into())),
            exec.resolve_node("/foo/".into())
        );
        assert_eq!(Node::DIR(foo), exec.resolve_node("/foo".into()).unwrap());
        assert_eq!(
            Node::DIR(bar),
            exec.resolve_node("/foo/bar".into()).unwrap()
        );
        assert_eq!(
            Node::FILE(boo),
            exec.resolve_node("/foo/bar/boo".into()).unwrap()
        );
        test_replay(exec.recording);
    }

    fn test_replay(workload: Workload) {
        let mut exec = AbstractExecutor::new();
        exec.replay(&workload).unwrap();
        assert_eq!(workload, exec.recording);
    }
}
