use std::collections::{HashMap, VecDeque};

use thiserror::Error;

use super::{
    flags::Mode,
    node::{Dir, DirIndex, File, FileDescriptor, FileIndex, Node},
    operation::Operation,
    pathname::{Name, PathName},
    workload::Workload,
};

type Result<T> = std::result::Result<T, FsError>;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum FsError {
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
    #[error("file at '{0}' was already opened")]
    FileAlreadyOpened(PathName),
    #[error("tried to rename '{0}' into subdirectory of itself '{1}'")]
    RenameToSubdirectoryError(PathName, PathName),
}

pub struct AbstractFS {
    pub dirs: Vec<Dir>,
    pub files: Vec<File>,

    pub descriptors: Vec<FileIndex>,

    pub recording: Workload,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AliveNodes {
    pub dirs: Vec<PathName>,
    pub files: Vec<(FileIndex, PathName)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Content;

impl Content {
    pub fn new() -> Self {
        Self {}
    }
}

impl AbstractFS {
    pub fn new() -> Self {
        AbstractFS {
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
            return Err(FsError::RootRemovalForbidden);
        }
        let (parent_path, name) = path.split();
        let parent_idx = self.resolve_dir(parent_path.to_owned())?;
        let parent = self.dir_mut(&parent_idx);
        if parent.children.remove(&name).is_none() {
            return Err(FsError::NotFound(path));
        }
        self.recording
            .push(Operation::REMOVE { path: path.clone() });
        Ok(())
    }

    pub fn mkdir(&mut self, path: PathName, mode: Mode) -> Result<DirIndex> {
        let (parent_path, name) = path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(FsError::NameAlreadyExists(path));
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
            return Err(FsError::NameAlreadyExists(path));
        }
        let file = File { descriptor: None };
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
            return Err(FsError::NameAlreadyExists(new_path));
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
        if old_path.is_prefix_of(&new_path) {
            return Err(FsError::RenameToSubdirectoryError(old_path, new_path));
        }
        if let Ok(dir_idx) = self.resolve_dir(new_path.clone()) {
            if !self.dir(&dir_idx).children.is_empty() {
                return Err(FsError::DirNotEmpty(new_path));
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
        let des = FileDescriptor(self.descriptors.len());
        let file_idx = self.resolve_file(path.clone())?;
        let file = self.file_mut(&file_idx);
        if file.descriptor.is_some() {
            return Err(FsError::FileAlreadyOpened(path));
        }
        file.descriptor = Some(des);
        self.descriptors.push(file_idx);
        self.recording.push(Operation::OPEN { path, des });
        Ok(des)
    }

    fn find_file_by_descriptor(&mut self, des: FileDescriptor) -> Result<&mut File> {
        let file_idx = self
            .descriptors
            .get(des.0)
            .ok_or(FsError::BadDescriptor(des, self.descriptors.len()))?
            .clone();
        let file = self.file_mut(&file_idx);
        if file.descriptor != Some(des) {
            return Err(FsError::DescriptorWasClosed(des));
        }
        Ok(file)
    }

    pub fn close(&mut self, des: FileDescriptor) -> Result<()> {
        let file = self.find_file_by_descriptor(des)?;
        file.descriptor = None;
        self.recording.push(Operation::CLOSE { des });
        Ok(())
    }

    pub fn read(&mut self, des: FileDescriptor, size: u64) -> Result<Content> {
        let file = self.find_file_by_descriptor(des)?;
        self.recording.push(Operation::READ { des, size });
        Ok(Content::new())
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
                Operation::OPEN { path, des: _ } => {
                    self.open(path.clone())?;
                }
                Operation::CLOSE { des } => {
                    self.close(des.clone())?;
                }
                Operation::READ { des, size } => {
                    self.read(des.clone(), size.clone())?;
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

    pub fn file(&self, idx: &FileIndex) -> &File {
        self.files.get(idx.0).unwrap()
    }

    fn file_mut(&mut self, idx: &FileIndex) -> &mut File {
        self.files.get_mut(idx.0).unwrap()
    }

    #[allow(dead_code)]
    fn root(&self) -> &Dir {
        self.dirs.get(0).unwrap()
    }

    pub fn resolve_node(&self, path: PathName) -> Result<Node> {
        if !path.is_valid() {
            return Err(FsError::InvalidPath(path));
        }
        let segments: Vec<&str> = path.segments();
        let mut last = Node::DIR(AbstractFS::root_index());
        let mut path = String::new();
        for segment in &segments {
            path.push_str("/");
            path.push_str(segment);
            let dir = match last {
                Node::DIR(dir_index) => self.dir(&dir_index),
                _ => return Err(FsError::NotADir(path.into())),
            };
            last = dir
                .children
                .get(segment.to_owned())
                .ok_or(FsError::NotFound(path.clone().into()))?
                .clone();
        }
        Ok(last)
    }

    pub fn resolve_file(&self, path: PathName) -> Result<FileIndex> {
        match self.resolve_node(path.clone())? {
            Node::FILE(idx) => Ok(idx),
            _ => Err(FsError::NotAFile(path)),
        }
    }

    pub fn resolve_dir(&self, path: PathName) -> Result<DirIndex> {
        match self.resolve_node(path.clone())? {
            Node::DIR(idx) => Ok(idx),
            _ => Err(FsError::NotADir(path)),
        }
    }

    pub fn root_index() -> DirIndex {
        DirIndex(0)
    }

    pub fn alive(&self) -> AliveNodes {
        let root = AbstractFS::root_index();
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
                    Node::FILE(idx) => {
                        alive.files.push((idx.clone(), path.join(name.to_owned())));
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
        let fs = AbstractFS::new();
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec![]
            },
            fs.alive()
        )
    }

    #[test]
    fn test_remove_root() {
        let mut fs = AbstractFS::new();
        assert_eq!(Err(FsError::RootRemovalForbidden), fs.remove("/".into()));
    }

    #[test]
    fn test_mkdir() {
        let mut fs = AbstractFS::new();
        let foo = fs.mkdir("/foobar".into(), vec![]).unwrap();
        assert_eq!(Node::DIR(foo), *fs.root().children.get("foobar").unwrap());
        assert_eq!(
            Workload {
                ops: vec![Operation::MKDIR {
                    path: "/foobar".into(),
                    mode: vec![],
                }],
            },
            fs.recording
        );
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/foobar".into()],
                files: vec![]
            },
            fs.alive()
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_mkdir_name_exists() {
        let mut fs = AbstractFS::new();
        fs.mkdir("/foobar".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::NameAlreadyExists("/foobar".into())),
            fs.mkdir("/foobar".into(), vec![])
        );
    }

    #[test]
    fn test_create() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foobar".into(), vec![]).unwrap();
        assert_eq!(Node::FILE(foo), *fs.root().children.get("foobar").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec![(foo, "/foobar".into())]
            },
            fs.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![Operation::CREATE {
                    path: "/foobar".into(),
                    mode: vec![],
                }]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_create_name_exists() {
        let mut fs = AbstractFS::new();
        fs.create("/foobar".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::NameAlreadyExists("/foobar".into())),
            fs.create("/foobar".into(), vec![])
        );
    }

    #[test]
    fn test_remove_file() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foobar".into(), vec![]).unwrap();
        let boo = fs.create("/boo".into(), vec![]).unwrap();

        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec![(foo, "/foobar".into()), (boo, "/boo".into())]
            },
            fs.alive()
        );

        fs.remove("/foobar".into()).unwrap();

        assert_eq!(1, fs.root().children.len());
        assert_eq!(Node::FILE(boo), *fs.root().children.get("boo").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec![(boo, "/boo".into())]
            },
            fs.alive()
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
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_hardlink() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        let bar = fs.mkdir("/bar".into(), vec![]).unwrap();
        let boo = fs.hardlink("/foo".into(), "/bar/boo".into()).unwrap();

        assert_eq!(foo, boo);
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/bar".into()],
                files: vec![(boo, "/bar/boo".into()), (foo, "/foo".into())]
            },
            fs.alive()
        );

        let root = fs.root();
        let bar_dir = fs.dir(&bar);
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
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_remove_hardlink() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        fs.hardlink("/foo".into(), "/bar".into()).unwrap();
        fs.remove("/bar".into()).unwrap();

        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec![(foo, "/foo".into())]
            },
            fs.alive()
        );

        assert_eq!(1, fs.root().children.len());

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
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_remove_hardlink_dir() {
        let mut fs = AbstractFS::new();
        let zero = fs.create("/0".into(), vec![]).unwrap();
        fs.mkdir("/1".into(), vec![]).unwrap();
        fs.mkdir("/1/2".into(), vec![]).unwrap();
        fs.hardlink("/0".into(), "/1/2/3".into()).unwrap();
        assert_eq!(Ok(zero), fs.resolve_file("/1/2/3".into()));
        fs.remove("/1".into()).unwrap();
        assert_eq!(
            Err(FsError::NotFound("/1".into())),
            fs.resolve_file("/1/2/3".into())
        );
        assert_eq!(Ok(zero), fs.resolve_file("/0".into()));
    }

    #[test]
    fn test_hardlink_name_exists() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        fs.create("/bar".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::NameAlreadyExists("/foo".into())),
            fs.hardlink("/bar".into(), "/foo".into())
        );
    }

    #[test]
    fn test_remove_dir() {
        let mut fs = AbstractFS::new();
        fs.mkdir("/foobar".into(), vec![]).unwrap();
        let boo = fs.mkdir("/boo".into(), vec![]).unwrap();

        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/boo".into(), "/foobar".into()],
                files: vec![]
            },
            fs.alive()
        );

        fs.remove("/foobar".into()).unwrap();

        assert_eq!(1, fs.root().children.len());
        assert_eq!(Node::DIR(boo), *fs.root().children.get("boo").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/boo".into()],
                files: vec![]
            },
            fs.alive()
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
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_remove_twice() {
        let mut fs = AbstractFS::new();
        fs.create("/0".into(), vec![]).unwrap();
        fs.remove("/0".into()).unwrap();
        assert_eq!(Err(FsError::NotFound("/0".into())), fs.remove("/0".into()))
    }

    #[test]
    fn test_rename_file() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        fs.rename("/foo".into(), "/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into()],
                files: vec![(foo, "/bar".into())]
            },
            fs.alive()
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
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_rename_dir() {
        let mut fs = AbstractFS::new();
        fs.mkdir("/foo".into(), vec![]).unwrap();
        fs.rename("/foo".into(), "/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec!["/".into(), "/bar".into()],
                files: vec![]
            },
            fs.alive()
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
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_rename_dir_non_empty() {
        let mut fs = AbstractFS::new();
        fs.mkdir("/foo".into(), vec![]).unwrap();
        fs.mkdir("/bar".into(), vec![]).unwrap();
        fs.create("/bar/baz".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::DirNotEmpty("/bar".into())),
            fs.rename("/foo".into(), "/bar".into())
        );
        fs.remove("/bar/baz".into()).unwrap();
        fs.rename("/foo".into(), "/bar".into()).unwrap();
    }

    #[test]
    fn test_rename_old_prefix() {
        let mut fs = AbstractFS::new();
        fs.mkdir("/0".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::RenameToSubdirectoryError(
                "/0".into(),
                "/0/1".into()
            )),
            fs.rename("/0".into(), "/0/1".into())
        );
    }

    #[test]
    fn test_open_close_file() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        let file = fs.file(&foo);
        assert_eq!(Some(des), file.descriptor);
        fs.close(des).unwrap();
        let file = fs.file(&foo);
        assert_eq!(None, file.descriptor);
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::OPEN {
                        path: "/foo".into(),
                        des
                    },
                    Operation::CLOSE { des }
                ]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_close_bad_descriptor() {
        let mut fs = AbstractFS::new();
        let des = FileDescriptor(0);
        assert_eq!(Err(FsError::BadDescriptor(des, 0)), fs.close(des));
    }

    #[test]
    fn test_close_twice() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        fs.close(des).unwrap();
        assert_eq!(Err(FsError::DescriptorWasClosed(des)), fs.close(des));
    }

    #[test]
    fn test_open_twice() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        fs.open("/foo".into()).unwrap();
        assert_eq!(
            Err(FsError::FileAlreadyOpened("/foo".into())),
            fs.open("/foo".into())
        );
    }

    #[test]
    fn test_read_bad_descriptor() {
        let mut fs = AbstractFS::new();
        let des = FileDescriptor(0);
        assert_eq!(Err(FsError::BadDescriptor(des, 0)), fs.read(des, 0));
    }

    #[test]
    fn test_read_closed() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        fs.close(des).unwrap();
        assert_eq!(Err(FsError::DescriptorWasClosed(des)), fs.read(des, 0));
    }

    #[test]
    fn test_read_empty() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        let content = fs.read(des, 1024).unwrap();
        fs.close(des).unwrap();

        assert_eq!(Content::new(), content);
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::OPEN {
                        path: "/foo".into(),
                        des
                    },
                    Operation::READ { des, size: 1024 },
                    Operation::CLOSE { des },
                ]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_resolve_node() {
        let mut fs = AbstractFS::new();
        assert_eq!(
            Node::DIR(AbstractFS::root_index()),
            fs.resolve_node("/".into()).unwrap()
        );
        let foo = fs.mkdir("/foo".into(), vec![]).unwrap();
        let bar = fs.mkdir("/foo/bar".into(), vec![]).unwrap();
        let boo = fs.create("/foo/bar/boo".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::InvalidPath("".into())),
            fs.resolve_node("".into())
        );
        assert_eq!(
            Err(FsError::InvalidPath("foo".into())),
            fs.resolve_node("foo".into())
        );
        assert_eq!(
            Err(FsError::InvalidPath("/foo/".into())),
            fs.resolve_node("/foo/".into())
        );
        assert_eq!(Node::DIR(foo), fs.resolve_node("/foo".into()).unwrap());
        assert_eq!(Node::DIR(bar), fs.resolve_node("/foo/bar".into()).unwrap());
        assert_eq!(
            Node::FILE(boo),
            fs.resolve_node("/foo/bar/boo".into()).unwrap()
        );
        test_replay(fs.recording);
    }

    fn test_replay(workload: Workload) {
        let mut fs = AbstractFS::new();
        fs.replay(&workload).unwrap();
        assert_eq!(workload, fs.recording);
    }
}
