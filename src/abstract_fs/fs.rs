use std::collections::{HashMap, VecDeque};

use thiserror::Error;

use super::{
    flags::Mode,
    node::{Content, Dir, DirIndex, File, FileDescriptor, FileDescriptorIndex, FileIndex, Node},
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
    BadDescriptor(FileDescriptorIndex, usize),
    #[error("descriptor '{0}' was already closed")]
    DescriptorWasClosed(FileDescriptorIndex),
    #[error("file at '{0}' was already opened")]
    FileAlreadyOpened(PathName),
    #[error("tried to rename '{0}' into subdirectory of itself '{1}'")]
    RenameToSubdirectoryError(PathName, PathName),
}

pub struct AbstractFS {
    pub dirs: Vec<Dir>,
    pub files: Vec<File>,

    pub descriptors: Vec<FileDescriptor>,

    pub recording: Workload,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AliveNodes {
    pub dirs: Vec<PathName>,
    pub files: Vec<(FileIndex, PathName)>,
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
        let file = File {
            descriptor: None,
            content: Content::new(),
        };
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

    pub fn open(&mut self, path: PathName) -> Result<FileDescriptorIndex> {
        let des = FileDescriptorIndex(self.descriptors.len());
        let file_idx = self.resolve_file(path.clone())?;
        let file = self.file_mut(&file_idx);
        if file.descriptor.is_some() {
            return Err(FsError::FileAlreadyOpened(path));
        }
        file.descriptor = Some(des);
        self.descriptors.push(FileDescriptor {
            file: file_idx,
            offset: 0,
        });
        self.recording.push(Operation::OPEN { path, des });
        Ok(des)
    }

    pub fn close(&mut self, des_idx: FileDescriptorIndex) -> Result<()> {
        let des = self.descriptor(&des_idx)?.clone();
        let file = self.file_mut(&des.file);
        if file.descriptor != Some(des_idx) {
            return Err(FsError::DescriptorWasClosed(des_idx));
        }
        file.descriptor = None;
        self.recording.push(Operation::CLOSE { des: des_idx });
        Ok(())
    }

    pub fn read(&mut self, des_idx: FileDescriptorIndex, size: u64) -> Result<Content> {
        let des = self.descriptor(&des_idx)?.clone();
        let offset = des.offset;
        let file = self.file_mut(&des.file);
        if file.descriptor != Some(des_idx) {
            return Err(FsError::DescriptorWasClosed(des_idx));
        }
        let content = file.content.read(offset, size);
        let file_size = file.content.size();
        let des = self.descriptor_mut(&des_idx)?;
        des.offset += content.size();
        assert!(des.offset <= file_size);
        self.recording.push(Operation::READ { des: des_idx, size });
        Ok(content)
    }

    pub fn write(
        &mut self,
        des_idx: FileDescriptorIndex,
        src_offset: u64,
        size: u64,
    ) -> Result<()> {
        let des = self.descriptor(&des_idx)?.clone();
        let file = self.file_mut(&des.file);
        if file.descriptor != Some(des_idx) {
            return Err(FsError::DescriptorWasClosed(des_idx));
        }
        let offset = des.offset;
        file.content.write(src_offset, offset, size);
        let file_size = file.content.size();
        let des = self.descriptor_mut(&des_idx)?;
        des.offset += size;
        assert!(des.offset <= file_size);
        self.recording.push(Operation::WRITE {
            des: des_idx,
            src_offset,
            size,
        });
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
                Operation::OPEN { path, des: _ } => {
                    self.open(path.clone())?;
                }
                Operation::CLOSE { des } => {
                    self.close(des.clone())?;
                }
                Operation::READ { des, size } => {
                    self.read(des.clone(), size.clone())?;
                }
                Operation::WRITE {
                    des,
                    src_offset,
                    size,
                } => {
                    self.write(des.clone(), src_offset.clone(), size.clone())?;
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

    fn descriptor(&self, idx: &FileDescriptorIndex) -> Result<&FileDescriptor> {
        Ok(self
            .descriptors
            .get(idx.0)
            .ok_or(FsError::BadDescriptor(idx.clone(), self.descriptors.len()))?)
    }

    fn descriptor_mut(&mut self, idx: &FileDescriptorIndex) -> Result<&mut FileDescriptor> {
        let len = self.descriptors.len();
        Ok(self
            .descriptors
            .get_mut(idx.0)
            .ok_or(FsError::BadDescriptor(idx.clone(), len))?)
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
    use crate::abstract_fs::node::SourceSlice;

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
        let des = FileDescriptorIndex(0);
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
        let des = FileDescriptorIndex(0);
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
    fn test_write_bad_descriptor() {
        let mut fs = AbstractFS::new();
        let des = FileDescriptorIndex(0);
        assert_eq!(Err(FsError::BadDescriptor(des, 0)), fs.write(des, 0, 0));
    }

    #[test]
    fn test_write_closed() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        fs.close(des).unwrap();
        assert_eq!(Err(FsError::DescriptorWasClosed(des)), fs.write(des, 0, 0));
    }

    #[test]
    fn test_write() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        fs.write(des, 999, 1024).unwrap();
        fs.close(des).unwrap();

        assert_eq!(
            vec![SourceSlice {
                from: 999,
                to: 999 + 1024 - 1
            }],
            fs.file(&foo).content.slices()
        );

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
                    Operation::WRITE {
                        des,
                        src_offset: 999,
                        size: 1024
                    },
                    Operation::CLOSE { des },
                ]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_write_rewrite() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        let des_1 = fs.open("/foo".into()).unwrap();
        fs.write(des_1, 13, 100).unwrap();
        fs.close(des_1).unwrap();
        let des_2 = fs.open("/foo".into()).unwrap();
        fs.write(des_2, 42, 55).unwrap();
        fs.close(des_2).unwrap();

        assert_eq!(
            vec![
                SourceSlice {
                    from: 42,
                    to: 42 + 55 - 1
                },
                SourceSlice {
                    from: 13 + 55,
                    to: 13 + 100 - 1
                }
            ],
            fs.file(&foo).content.slices()
        );

        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::OPEN {
                        path: "/foo".into(),
                        des: des_1
                    },
                    Operation::WRITE {
                        des: des_1,
                        src_offset: 13,
                        size: 100
                    },
                    Operation::CLOSE { des: des_1 },
                    Operation::OPEN {
                        path: "/foo".into(),
                        des: des_2
                    },
                    Operation::WRITE {
                        des: des_2,
                        src_offset: 42,
                        size: 55
                    },
                    Operation::CLOSE { des: des_2 },
                ]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_read() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        let des_write = fs.open("/foo".into()).unwrap();
        fs.write(des_write, 13, 100).unwrap();
        fs.write(des_write, 42, 55).unwrap();
        fs.close(des_write).unwrap();
        let des_read = fs.open("/foo".into()).unwrap();
        assert_eq!(
            Vec::<SourceSlice>::new(),
            fs.read(des_read, 0).unwrap().slices()
        );
        assert_eq!(
            vec![SourceSlice {
                from: 13,
                to: 13 + 10 - 1
            }],
            fs.read(des_read, 10).unwrap().slices()
        );
        assert_eq!(
            vec![SourceSlice {
                from: (13 + 10),
                to: (13 + 10) + 10 - 1
            }],
            fs.read(des_read, 10).unwrap().slices()
        );
        assert_eq!(
            vec![
                SourceSlice {
                    from: (13 + 20),
                    to: 13 + 100 - 1
                },
                SourceSlice {
                    from: 42,
                    to: 42 + 55 - 1
                },
            ],
            fs.read(des_read, 1024).unwrap().slices()
        );
        fs.close(des_read).unwrap();

        assert_eq!(
            Workload {
                ops: vec![
                    Operation::CREATE {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::OPEN {
                        path: "/foo".into(),
                        des: des_write
                    },
                    Operation::WRITE {
                        des: des_write,
                        src_offset: 13,
                        size: 100
                    },
                    Operation::CLOSE { des: des_write },
                    Operation::OPEN {
                        path: "/foo".into(),
                        des: des_read
                    },
                    Operation::READ {
                        des: des_read,
                        size: 0
                    },
                    Operation::READ {
                        des: des_read,
                        size: 10
                    },
                    Operation::READ {
                        des: des_read,
                        size: 10
                    },
                    Operation::READ {
                        des: des_read,
                        size: 1024
                    },
                    Operation::CLOSE { des: des_read },
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
