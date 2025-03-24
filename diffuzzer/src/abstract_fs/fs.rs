/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::collections::{HashMap, VecDeque};

use thiserror::Error;

use super::{
    content::{Content, ContentError},
    flags::Mode,
    node::{
        Dir, DirIndex, File, FileDescriptor, FileDescriptorIndex, FileIndex, Node, Symlink,
        SymlinkIndex,
    },
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
    #[error("loop exists in symbolic links encountered during path resolution")]
    LoopExists(PathName),
    #[error(transparent)]
    ContentError(#[from] ContentError),
}

/// Abstract model of filesystem that approximates filesystem functions.
///
/// All file nodes are stored as vectors and can be accessed using indicies (similar to inodes).
/// Deleted nodes are not removed from the vectors to keep indicies unchanged.
pub struct AbstractFS {
    pub dirs: Vec<Dir>,
    pub files: Vec<File>,
    pub symlinks: Vec<Symlink>,
    pub descriptors: Vec<FileDescriptor>,
    /// Every succesful operation is recorded and can be replayed from scratch.
    pub recording: Workload,
}

/// File nodes that are accessible from root (not deleted).
#[derive(Debug, PartialEq, Eq)]
pub struct AliveNodes {
    pub dirs: Vec<(DirIndex, PathName)>,
    pub files: Vec<(FileIndex, PathName)>,
    pub symlinks: Vec<PathName>,
}

const MAX_SYMLINK_FOLLOW: u8 = 2;

impl AbstractFS {
    pub fn new() -> Self {
        AbstractFS {
            dirs: vec![Dir {
                children: HashMap::new(),
            }],
            files: vec![],
            descriptors: vec![],
            symlinks: vec![],
            recording: Workload::new(),
        }
    }

    /// Removes node, similar to `unlink` (for files) and `rmdir` (for dirs).
    pub fn remove(&mut self, path: PathName) -> Result<()> {
        if path.is_root() {
            return Err(FsError::RootRemovalForbidden);
        }
        let (parent_path, name) = path.split();
        let (_, parent_idx) = self.resolve_dir(parent_path.to_owned())?;
        let parent = self.dir_mut(&parent_idx);
        if parent.children.remove(&name).is_none() {
            return Err(FsError::NotFound(path));
        }
        self.recording
            .push(Operation::Remove { path: path.clone() });
        Ok(())
    }

    /// Creates an empty directory, similar to `mkdir`.
    pub fn mkdir(&mut self, path: PathName, mode: Mode) -> Result<DirIndex> {
        let (parent_path, name) = path.split();
        let (_, parent) = self.resolve_dir(parent_path.to_owned())?;
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
            .insert(name, Node::Dir(dir_idx));
        self.recording.push(Operation::MkDir { path, mode });
        Ok(dir_idx)
    }

    /// Creates an empty file, similar to `creat` but without open file descriptor.
    pub fn create(&mut self, path: PathName, mode: Mode) -> Result<FileIndex> {
        let (parent_path, name) = path.split();
        let (_, parent) = self.resolve_dir(parent_path.to_owned())?;
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
            .insert(name.clone(), Node::File(file_idx));
        self.recording.push(Operation::Create { path, mode });
        Ok(file_idx)
    }

    /// Creates a "hard" link from one file to another, similar to `link`.
    /// Both files refer to the same node (in the file tree) but with different names.
    pub fn hardlink(&mut self, old_path: PathName, new_path: PathName) -> Result<FileIndex> {
        let (_, old_file) = self.resolve_file(old_path.clone())?;
        let (parent_path, name) = new_path.split();
        let (_, parent) = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(FsError::NameAlreadyExists(new_path));
        }
        let parent_dir = self.dir_mut(&parent);
        parent_dir
            .children
            .insert(name.clone(), Node::File(old_file.to_owned()));
        self.recording
            .push(Operation::Hardlink { old_path, new_path });
        Ok(old_file.to_owned())
    }

    pub fn symlink(&mut self, target: PathName, linkpath: PathName) -> Result<SymlinkIndex> {
        let (parent_path, name) = linkpath.split();
        let (_, parent) = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(FsError::NameAlreadyExists(linkpath));
        }
        let symlink = Symlink {
            target: target.clone(),
        };
        let sym_idx = SymlinkIndex(self.symlinks.len());
        self.symlinks.push(symlink);
        self.dir_mut(&parent)
            .children
            .insert(name.clone(), Node::Symlink(sym_idx));
        self.recording.push(Operation::Symlink { target, linkpath });
        Ok(sym_idx)
    }

    /// Renames a file, moving it between directories if required, similar to `rename`.
    pub fn rename(&mut self, old_path: PathName, new_path: PathName) -> Result<Node> {
        if let Ok((_, dir_idx)) = self.resolve_dir(new_path.clone()) {
            if !self.dir(&dir_idx).children.is_empty() {
                return Err(FsError::DirNotEmpty(new_path));
            }
        }

        let (_, node) = self.resolve_node(old_path.clone(), false)?;
        let (parent_path, name) = new_path.split();
        let (old_dirs, parent) = self.resolve_dir(parent_path.to_owned())?;

        if let Node::Dir(old_idx) = node {
            if old_dirs.contains(&old_idx) || parent == old_idx {
                return Err(FsError::RenameToSubdirectoryError(old_path, new_path));
            }
        }

        let parent_dir = self.dir_mut(&parent);
        parent_dir.children.insert(name.clone(), node.clone());

        let (parent_path, name) = old_path.split();
        let (_, parent) = self.resolve_dir(parent_path.to_owned())?;
        let parent_dir = self.dir_mut(&parent);
        parent_dir.children.remove(&name);

        self.recording
            .push(Operation::Rename { old_path, new_path });
        Ok(node)
    }

    /// Opens a file and returns the file descriptor, similar to `open`.
    ///
    /// TODO: flags
    pub fn open(&mut self, path: PathName) -> Result<FileDescriptorIndex> {
        let des = FileDescriptorIndex(self.descriptors.len());
        let (_, file_idx) = self.resolve_file(path.clone())?;
        let file = self.file_mut(&file_idx);
        if file.descriptor.is_some() {
            return Err(FsError::FileAlreadyOpened(path));
        }
        file.descriptor = Some(des);
        self.descriptors.push(FileDescriptor {
            file: file_idx,
            offset: 0,
        });
        self.recording.push(Operation::Open { path, des });
        Ok(des)
    }

    /// Closes a file using the file descriptor, similar to `close`.
    pub fn close(&mut self, des_idx: FileDescriptorIndex) -> Result<()> {
        let des = self.descriptor(&des_idx)?.clone();
        let file = self.file_mut(&des.file);
        if file.descriptor != Some(des_idx) {
            return Err(FsError::DescriptorWasClosed(des_idx));
        }
        file.descriptor = None;
        self.recording.push(Operation::Close { des: des_idx });
        Ok(())
    }

    /// Reads content of file using the file descriptor of specified size, similar to `read`.
    /// Read position is managed by descriptor.
    pub fn read(&mut self, des_idx: FileDescriptorIndex, size: u64) -> Result<Content> {
        let des = self.descriptor(&des_idx)?.clone();
        let offset = des.offset;
        let file = self.file_mut(&des.file);
        if file.descriptor != Some(des_idx) {
            return Err(FsError::DescriptorWasClosed(des_idx));
        }
        let content = file.content.read(offset, size)?;
        let file_size = file.content.size();
        let des = self.descriptor_mut(&des_idx)?;
        des.offset += content.size();
        assert!(
            des.offset <= file_size,
            "offset: {}, size: {}",
            des.offset,
            file_size
        );
        self.recording.push(Operation::Read { des: des_idx, size });
        Ok(content)
    }

    /// Writes slice of "source" data using the file descriptor, similar to `write`.
    /// Write position is managed by descriptor.
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
        file.content.write(src_offset, offset, size)?;
        let file_size = file.content.size();
        let des = self.descriptor_mut(&des_idx)?;
        des.offset += size;
        assert!(
            des.offset <= file_size,
            "offset: {}, size: {}",
            des.offset,
            file_size
        );
        self.recording.push(Operation::Write {
            des: des_idx,
            src_offset,
            size,
        });
        Ok(())
    }

    /// No-op, sync file state with storage device, similar to `fsync`.
    pub fn fsync(&mut self, des_idx: FileDescriptorIndex) -> Result<()> {
        let des = self.descriptor(&des_idx)?.clone();
        let file = self.file_mut(&des.file);
        if file.descriptor != Some(des_idx) {
            return Err(FsError::DescriptorWasClosed(des_idx));
        }
        self.recording.push(Operation::FSync { des: des_idx });
        Ok(())
    }

    /// Replay operations from workload. Does not reset the state.
    pub fn replay(&mut self, workload: &Workload) -> Result<()> {
        for op in &workload.ops {
            match op {
                Operation::MkDir { path, mode } => {
                    self.mkdir(path.clone(), mode.clone())?;
                }
                Operation::Create { path, mode } => {
                    self.create(path.clone(), mode.clone())?;
                }
                Operation::Remove { path } => self.remove(path.clone())?,
                Operation::Hardlink { old_path, new_path } => {
                    self.hardlink(old_path.clone(), new_path.clone())?;
                }
                Operation::Rename { old_path, new_path } => {
                    self.rename(old_path.clone(), new_path.clone())?;
                }
                Operation::Open { path, des: _ } => {
                    self.open(path.clone())?;
                }
                Operation::Close { des } => {
                    self.close(*des)?;
                }
                Operation::Read { des, size } => {
                    self.read(*des, *size)?;
                }
                Operation::Write {
                    des,
                    src_offset,
                    size,
                } => {
                    self.write(*des, *src_offset, *size)?;
                }
                Operation::FSync { des } => {
                    self.fsync(*des)?;
                }
                Operation::Symlink { target, linkpath } => {
                    self.symlink(target.clone(), linkpath.clone())?;
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

    fn sym(&self, idx: &SymlinkIndex) -> &Symlink {
        self.symlinks.get(idx.0).unwrap()
    }

    #[allow(dead_code)]
    fn root(&self) -> &Dir {
        self.dirs.first().unwrap()
    }

    fn descriptor(&self, idx: &FileDescriptorIndex) -> Result<&FileDescriptor> {
        self.descriptors
            .get(idx.0)
            .ok_or(FsError::BadDescriptor(*idx, self.descriptors.len()))
    }

    fn descriptor_mut(&mut self, idx: &FileDescriptorIndex) -> Result<&mut FileDescriptor> {
        let len = self.descriptors.len();
        self.descriptors
            .get_mut(idx.0)
            .ok_or(FsError::BadDescriptor(*idx, len))
    }

    pub fn resolve_node(
        &self,
        path: PathName,
        follow_symlinks: bool,
    ) -> Result<(Vec<DirIndex>, Node)> {
        self.resolve_node_rec(path, follow_symlinks, vec![])
    }

    pub fn resolve_node_rec(
        &self,
        path: PathName,
        follow_symlinks: bool,
        mut visited_symlinks: Vec<SymlinkIndex>,
    ) -> Result<(Vec<DirIndex>, Node)> {
        if !path.is_valid() {
            return Err(FsError::InvalidPath(path));
        }
        let mut dirs = vec![];
        let segments = path.segments();
        let mut last = Node::Dir(AbstractFS::root_index());
        let mut path = String::new();
        for segment in &segments {
            path.push('/');
            path.push_str(segment);
            let dir = match last {
                Node::Dir(idx) => {
                    dirs.push(idx);
                    self.dir(&idx)
                }
                Node::Symlink(idx) => {
                    let target = self.sym(&idx).target.clone();
                    let (mut rec_dirs, idx) = self.resolve_dir(target)?;
                    dirs.append(&mut rec_dirs);
                    dirs.push(idx);
                    self.dir(&idx)
                }
                _ => return Err(FsError::NotADir(path.into())),
            };
            last = dir
                .children
                .get(segment.to_owned())
                .ok_or(FsError::NotFound(path.clone().into()))?
                .clone();
        }
        match last {
            Node::Symlink(idx) if follow_symlinks => {
                if visited_symlinks.contains(&idx) {
                    return Err(FsError::LoopExists(path.into()));
                }
                let target = self.sym(&idx).target.clone();
                visited_symlinks.push(idx);
                let (mut rec_dirs, last) =
                    self.resolve_node_rec(target, follow_symlinks, visited_symlinks)?;
                dirs.append(&mut rec_dirs);
                Ok((dirs, last))
            }
            _ => Ok((dirs, last)),
        }
    }

    pub fn resolve_file(&self, path: PathName) -> Result<(Vec<DirIndex>, FileIndex)> {
        match self.resolve_node(path.clone(), true)? {
            (dirs, Node::File(idx)) => Ok((dirs, idx)),
            _ => Err(FsError::NotAFile(path)),
        }
    }

    pub fn resolve_dir(&self, path: PathName) -> Result<(Vec<DirIndex>, DirIndex)> {
        match self.resolve_node(path.clone(), true)? {
            (dirs, Node::Dir(idx)) => Ok((dirs, idx)),
            _ => Err(FsError::NotADir(path)),
        }
    }

    pub fn root_index() -> DirIndex {
        DirIndex(0)
    }

    /// Get nodes that are considired "alive" (accessible from root)
    pub fn alive(&self) -> AliveNodes {
        let root = AbstractFS::root_index();
        let mut alive = AliveNodes {
            dirs: vec![],
            files: vec![],
            symlinks: vec![],
        };
        let mut queue = VecDeque::new();
        queue.push_back(("/".into(), root));
        alive.dirs.push((Self::root_index(), "/".into()));

        // Because symbolic links can loop back, we follow them only a few times.
        for _ in 1..=MAX_SYMLINK_FOLLOW {
            let follow_queue = self.alive_follow_once(&mut alive, queue);
            queue = follow_queue;
        }

        alive.dirs.sort();
        alive.files.sort();
        alive.symlinks.sort();
        alive
    }

    /// Breadth-first search, follow symbolic links once
    fn alive_follow_once(
        &self,
        alive: &mut AliveNodes,
        mut queue: VecDeque<(PathName, DirIndex)>,
    ) -> VecDeque<(PathName, DirIndex)> {
        let mut follow_next = VecDeque::new();
        while let Some((dir_path, idx)) = queue.pop_front() {
            let dir = self.dir(&idx);
            for (child_name, node) in dir.children.iter() {
                match node {
                    Node::Dir(idx) => {
                        let path = dir_path.join(child_name.to_owned());
                        queue.push_back((path.clone(), *idx));
                        alive.dirs.push((idx.clone(), path.clone()));
                    }
                    Node::File(idx) => {
                        alive
                            .files
                            .push((*idx, dir_path.join(child_name.to_owned())));
                    }
                    Node::Symlink(idx) => {
                        alive.symlinks.push(dir_path.join(child_name.to_owned()));
                        let follow_path = self.sym(&idx).target.clone();
                        match self.resolve_node(follow_path, true) {
                            Ok((_, Node::File(idx))) => {
                                alive
                                    .files
                                    .push((idx, dir_path.join(child_name.to_owned())));
                            }
                            Ok((_, Node::Dir(idx))) => {
                                let path = dir_path.join(child_name.to_owned());
                                follow_next.push_back((path.clone(), idx));
                                alive.dirs.push((idx, path.clone()));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        follow_next
    }
}

#[cfg(test)]
mod tests {
    use crate::abstract_fs::content::SourceSlice;

    use super::*;

    #[test]
    fn test_init_root() {
        let fs = AbstractFS::new();
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![],
                symlinks: vec![],
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
        assert_eq!(Node::Dir(foo), *fs.root().children.get("foobar").unwrap());
        assert_eq!(
            Workload {
                ops: vec![Operation::MkDir {
                    path: "/foobar".into(),
                    mode: vec![],
                }],
            },
            fs.recording
        );
        assert_eq!(
            AliveNodes {
                dirs: vec![
                    (AbstractFS::root_index(), "/".into()),
                    (foo, "/foobar".into())
                ],
                files: vec![],
                symlinks: vec![],
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
        assert_eq!(Node::File(foo), *fs.root().children.get("foobar").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(foo, "/foobar".into())],
                symlinks: vec![],
            },
            fs.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![Operation::Create {
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
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(foo, "/foobar".into()), (boo, "/boo".into())],
                symlinks: vec![],
            },
            fs.alive()
        );

        fs.remove("/foobar".into()).unwrap();

        assert_eq!(1, fs.root().children.len());
        assert_eq!(Node::File(boo), *fs.root().children.get("boo").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(boo, "/boo".into())],
                symlinks: vec![],
            },
            fs.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::Create {
                        path: "/foobar".into(),
                        mode: vec![],
                    },
                    Operation::Create {
                        path: "/boo".into(),
                        mode: vec![],
                    },
                    Operation::Remove {
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
                dirs: vec![(AbstractFS::root_index(), "/".into()), (bar, "/bar".into())],
                files: vec![(boo, "/bar/boo".into()), (foo, "/foo".into())],
                symlinks: vec![],
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
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![],
                    },
                    Operation::MkDir {
                        path: "/bar".into(),
                        mode: vec![],
                    },
                    Operation::Hardlink {
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
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(foo, "/foo".into())],
                symlinks: vec![],
            },
            fs.alive()
        );

        assert_eq!(1, fs.root().children.len());

        assert_eq!(
            Workload {
                ops: vec![
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![],
                    },
                    Operation::Hardlink {
                        old_path: "/foo".into(),
                        new_path: "/bar".into(),
                    },
                    Operation::Remove {
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
        let root = AbstractFS::root_index();
        let zero = fs.create("/0".into(), vec![]).unwrap();
        let one = fs.mkdir("/1".into(), vec![]).unwrap();
        let two = fs.mkdir("/1/2".into(), vec![]).unwrap();
        fs.hardlink("/0".into(), "/1/2/3".into()).unwrap();
        assert_eq!(
            Ok((vec![root, one, two], zero)),
            fs.resolve_file("/1/2/3".into())
        );
        fs.remove("/1".into()).unwrap();
        assert_eq!(
            Err(FsError::NotFound("/1".into())),
            fs.resolve_file("/1/2/3".into())
        );
        assert_eq!(Ok((vec![root], zero)), fs.resolve_file("/0".into()));
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
        let foo = fs.mkdir("/foobar".into(), vec![]).unwrap();
        let boo = fs.mkdir("/boo".into(), vec![]).unwrap();

        assert_eq!(
            AliveNodes {
                dirs: vec![
                    (AbstractFS::root_index(), "/".into()),
                    (foo, "/foobar".into()),
                    (boo, "/boo".into()),
                ],
                files: vec![],
                symlinks: vec![],
            },
            fs.alive()
        );

        fs.remove("/foobar".into()).unwrap();

        assert_eq!(1, fs.root().children.len());
        assert_eq!(Node::Dir(boo), *fs.root().children.get("boo").unwrap());
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into()), (boo, "/boo".into())],
                files: vec![],
                symlinks: vec![],
            },
            fs.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::MkDir {
                        path: "/foobar".into(),
                        mode: vec![],
                    },
                    Operation::MkDir {
                        path: "/boo".into(),
                        mode: vec![],
                    },
                    Operation::Remove {
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
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(foo, "/bar".into())],
                symlinks: vec![],
            },
            fs.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Rename {
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
        let foo = fs.mkdir("/foo".into(), vec![]).unwrap();
        fs.rename("/foo".into(), "/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into()), (foo, "/bar".into())],
                files: vec![],
                symlinks: vec![],
            },
            fs.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::MkDir {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Rename {
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
    fn test_rename_to_subdirectory() {
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
    fn test_rename_to_subdirectory_symlink() {
        let mut fs = AbstractFS::new();
        fs.mkdir("/0".into(), vec![]).unwrap();
        fs.symlink("/0".into(), "/symlink".into()).unwrap();
        assert_eq!(
            Err(FsError::RenameToSubdirectoryError(
                "/0".into(),
                "/symlink/1".into()
            )),
            fs.rename("/0".into(), "/symlink/1".into())
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
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Open {
                        path: "/foo".into(),
                        des
                    },
                    Operation::Close { des }
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
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Open {
                        path: "/foo".into(),
                        des
                    },
                    Operation::Read { des, size: 1024 },
                    Operation::Close { des },
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
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Open {
                        path: "/foo".into(),
                        des
                    },
                    Operation::Write {
                        des,
                        src_offset: 999,
                        size: 1024
                    },
                    Operation::Close { des },
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
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Open {
                        path: "/foo".into(),
                        des: des_1
                    },
                    Operation::Write {
                        des: des_1,
                        src_offset: 13,
                        size: 100
                    },
                    Operation::Close { des: des_1 },
                    Operation::Open {
                        path: "/foo".into(),
                        des: des_2
                    },
                    Operation::Write {
                        des: des_2,
                        src_offset: 42,
                        size: 55
                    },
                    Operation::Close { des: des_2 },
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
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Open {
                        path: "/foo".into(),
                        des: des_write
                    },
                    Operation::Write {
                        des: des_write,
                        src_offset: 13,
                        size: 100
                    },
                    Operation::Write {
                        des: des_write,
                        src_offset: 42,
                        size: 55
                    },
                    Operation::Close { des: des_write },
                    Operation::Open {
                        path: "/foo".into(),
                        des: des_read
                    },
                    Operation::Read {
                        des: des_read,
                        size: 0
                    },
                    Operation::Read {
                        des: des_read,
                        size: 10
                    },
                    Operation::Read {
                        des: des_read,
                        size: 10
                    },
                    Operation::Read {
                        des: des_read,
                        size: 1024
                    },
                    Operation::Close { des: des_read },
                ]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_fsync_bad_descriptor() {
        let mut fs = AbstractFS::new();
        let des = FileDescriptorIndex(0);
        assert_eq!(Err(FsError::BadDescriptor(des, 0)), fs.fsync(des));
    }

    #[test]
    fn test_fsync_closed() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        fs.close(des).unwrap();
        assert_eq!(Err(FsError::DescriptorWasClosed(des)), fs.fsync(des));
    }

    #[test]
    fn test_fsync() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        let des = fs.open("/foo".into()).unwrap();
        fs.fsync(des).unwrap();
        fs.close(des).unwrap();

        assert_eq!(
            Workload {
                ops: vec![
                    Operation::Create {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Open {
                        path: "/foo".into(),
                        des
                    },
                    Operation::FSync { des },
                    Operation::Close { des },
                ]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_symlink() {
        let mut fs = AbstractFS::new();
        let foo = fs.mkdir("/foo".into(), vec![]).unwrap();
        let bar = fs.create("/foo/bar".into(), vec![]).unwrap();
        fs.symlink("/foo".into(), "/baz".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![
                    (AbstractFS::root_index(), "/".into()),
                    (foo, "/baz".into()),
                    (foo, "/foo".into())
                ],
                files: vec![(bar, "/baz/bar".into()), (bar, "/foo/bar".into())],
                symlinks: vec!["/baz".into()],
            },
            fs.alive()
        );
        assert_eq!(
            Workload {
                ops: vec![
                    Operation::MkDir {
                        path: "/foo".into(),
                        mode: vec![]
                    },
                    Operation::Create {
                        path: "/foo/bar".into(),
                        mode: vec![]
                    },
                    Operation::Symlink {
                        target: "/foo".into(),
                        linkpath: "/baz".into(),
                    }
                ]
            },
            fs.recording
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_symlink_name_exists() {
        let mut fs = AbstractFS::new();
        fs.create("/foo".into(), vec![]).unwrap();
        fs.create("/bar".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::NameAlreadyExists("/bar".into())),
            fs.symlink("/foo".into(), "/bar".into()),
        );
    }

    #[test]
    fn test_symlink_recursion() {
        let mut fs = AbstractFS::new();
        let foo = fs.mkdir("/foo".into(), vec![]).unwrap();
        fs.symlink("/foo".into(), "/foo/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![
                    (AbstractFS::root_index(), "/".into()),
                    (foo, "/foo".into()),
                    (foo, "/foo/bar".into()),
                    (foo, "/foo/bar/bar".into()),
                    (foo, "/foo/bar/bar/bar".into())
                ],
                files: vec![],
                symlinks: vec![
                    "/foo/bar".into(),
                    "/foo/bar/bar".into(),
                    "/foo/bar/bar/bar".into(),
                ]
            },
            fs.alive()
        );
    }

    #[test]
    fn test_symlink_to_symlink() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        fs.symlink("/foo".into(), "/bar".into()).unwrap();
        fs.symlink("/bar".into(), "/boo".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into()),],
                files: vec![
                    (foo, "/bar".into()),
                    (foo, "/boo".into()),
                    (foo, "/foo".into()),
                ],
                symlinks: vec!["/bar".into(), "/boo".into()]
            },
            fs.alive()
        );
    }

    #[test]
    fn test_symlink_to_symlink_recursive() {
        let mut fs = AbstractFS::new();
        fs.symlink("/foo".into(), "/bar".into()).unwrap();
        fs.symlink("/bar".into(), "/foo".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into()),],
                files: vec![],
                symlinks: vec!["/bar".into(), "/foo".into()]
            },
            fs.alive()
        );
    }

    #[test]
    fn test_remove_symlink() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        fs.symlink("/foo".into(), "/bar".into()).unwrap();
        fs.remove("/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(foo, "/foo".into())],
                symlinks: vec![],
            },
            fs.alive()
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_rename_symlink() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        fs.symlink("/foo".into(), "/bar".into()).unwrap();
        fs.rename("/bar".into(), "/baz".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(foo, "/baz".into()), (foo, "/foo".into())],
                symlinks: vec!["/baz".into()],
            },
            fs.alive()
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_rename_symlink_overwrite() {
        let mut fs = AbstractFS::new();
        let foo = fs.create("/foo".into(), vec![]).unwrap();
        fs.symlink("/foo".into(), "/bar".into()).unwrap();
        fs.rename("/foo".into(), "/bar".into()).unwrap();
        assert_eq!(
            AliveNodes {
                dirs: vec![(AbstractFS::root_index(), "/".into())],
                files: vec![(foo, "/bar".into())],
                symlinks: vec![],
            },
            fs.alive()
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_resolve_node() {
        let mut fs = AbstractFS::new();
        let root = AbstractFS::root_index();
        assert_eq!(
            (vec![], Node::Dir(root)),
            fs.resolve_node("/".into(), true).unwrap()
        );
        let foo = fs.mkdir("/foo".into(), vec![]).unwrap();
        let bar = fs.mkdir("/foo/bar".into(), vec![]).unwrap();
        let boo = fs.create("/foo/bar/boo".into(), vec![]).unwrap();
        assert_eq!(
            Err(FsError::InvalidPath("".into())),
            fs.resolve_node("".into(), true)
        );
        assert_eq!(
            Err(FsError::InvalidPath("foo".into())),
            fs.resolve_node("foo".into(), true)
        );
        assert_eq!(
            Err(FsError::InvalidPath("/foo/".into())),
            fs.resolve_node("/foo/".into(), true)
        );
        assert_eq!(
            (vec![root], Node::Dir(foo)),
            fs.resolve_node("/foo".into(), true).unwrap()
        );
        assert_eq!(
            (vec![root, foo], Node::Dir(bar)),
            fs.resolve_node("/foo/bar".into(), true).unwrap()
        );
        assert_eq!(
            (vec![root, foo, bar], Node::File(boo)),
            fs.resolve_node("/foo/bar/boo".into(), true).unwrap()
        );
        test_replay(fs.recording);
    }

    #[test]
    fn test_resolve_node_symlinks() {
        let mut fs = AbstractFS::new();
        let root = AbstractFS::root_index();
        let foo = fs.mkdir("/foo".into(), vec![]).unwrap();
        let bar = fs.create("/foo/bar".into(), vec![]).unwrap();
        let foos = fs.symlink("/foo".into(), "/foos".into()).unwrap();
        assert_eq!(
            (vec![root], Node::Dir(foo)),
            fs.resolve_node("/foo".into(), true).unwrap()
        );
        assert_eq!(
            (vec![root, root], Node::Dir(foo)),
            fs.resolve_node("/foos".into(), true).unwrap()
        );
        assert_eq!(
            (vec![root], Node::Symlink(foos)),
            fs.resolve_node("/foos".into(), false).unwrap()
        );
        assert_eq!(
            (vec![root, foo], Node::File(bar)),
            fs.resolve_node("/foo/bar".into(), true).unwrap()
        );
        assert_eq!(
            (vec![root, root, foo], Node::File(bar)),
            fs.resolve_node("/foos/bar".into(), true).unwrap()
        );
        // Always follow symlinks in dirname part of the path.
        assert_eq!(
            (vec![root, root, foo], Node::File(bar)),
            fs.resolve_node("/foos/bar".into(), false).unwrap()
        );
        test_replay(fs.recording);
    }

    fn test_replay(workload: Workload) {
        let mut fs = AbstractFS::new();
        fs.replay(&workload).unwrap();
        assert_eq!(workload, fs.recording);
    }
}
