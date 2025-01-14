use std::collections::{HashMap, HashSet, VecDeque};

use thiserror::Error;

use super::{
    flags::Mode,
    node::{Dir, DirIndex, File, FileIndex, Node},
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
}

pub struct AbstractExecutor {
    pub dirs: Vec<Dir>,
    pub files: Vec<File>,
    pub nodes_created: usize,
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
                parent: None,
                children: HashMap::new(),
            }],
            files: vec![],
            nodes_created: 0,
            recording: Workload::new(),
        }
    }

    pub fn remove(&mut self, path: PathName) -> Result<()> {
        let (parent_path, name) = path.split();
        let node = &self.resolve_node(path.clone())?;
        let parent_idx = self.resolve_dir(parent_path.to_owned())?;
        self.recording
            .push(Operation::REMOVE { path: path.clone() });
        let parent = self.dir_mut(&parent_idx);
        parent.children.remove(&name);
        match node {
            Node::DIR(to_remove_idx) => {
                if *to_remove_idx == AbstractExecutor::root_index() {
                    return Err(ExecutorError::RootRemovalForbidden);
                }
                let mut queue: VecDeque<(DirIndex, Node)> = VecDeque::new();
                let to_remove = self.dir_mut(to_remove_idx);
                for (_, node) in to_remove.children.iter() {
                    queue.push_back((to_remove_idx.clone(), node.clone()));
                }
                to_remove.parent = None;
                to_remove.children.clear();
                while let Some((parent, node)) = queue.pop_front() {
                    match node {
                        Node::DIR(to_remove_idx) => {
                            let to_remove = self.dir_mut(&to_remove_idx);
                            for (_, node) in to_remove.children.iter() {
                                queue.push_back((to_remove_idx.clone(), node.clone()));
                            }
                            to_remove.parent = None;
                            to_remove.children.clear();
                        }
                        Node::FILE(file_idx) => {
                            let file = self.file_mut(&file_idx);
                            file.parents.remove(&parent);
                        }
                    }
                }
            }
            Node::FILE(to_remove) => {
                let another_exists = parent.children.iter().any(|(_, node)| match node {
                    Node::FILE(another) if another == to_remove => true,
                    _ => false,
                });
                if !another_exists {
                    let to_remove = self.file_mut(to_remove);
                    to_remove.parents.remove(&parent_idx);
                }
            }
        }
        Ok(())
    }

    pub fn mkdir(&mut self, path: PathName, mode: Mode) -> Result<DirIndex> {
        let (parent_path, name) = path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(ExecutorError::NameAlreadyExists(
                self.resolve_dir_path(&parent).join(name),
            ));
        }
        let dir = Dir {
            parent: Some(parent.clone()),
            children: HashMap::new(),
        };
        let dir_idx = DirIndex(self.dirs.len());
        self.dirs.push(dir);
        self.dir_mut(&parent)
            .children
            .insert(name, Node::DIR(dir_idx));
        self.recording.push(Operation::MKDIR {
            path: self.resolve_dir_path(&dir_idx),
            mode,
        });
        self.nodes_created += 1;
        Ok(dir_idx)
    }

    pub fn create(&mut self, path: PathName, mode: Mode) -> Result<FileIndex> {
        let (parent_path, name) = path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(ExecutorError::NameAlreadyExists(
                self.resolve_dir_path(&parent).join(name),
            ));
        }
        let mut parents = HashSet::new();
        parents.insert(parent.to_owned());
        let file = File { parents };
        let file_idx = FileIndex(self.files.len());
        self.files.push(file);
        self.dir_mut(&parent)
            .children
            .insert(name.clone(), Node::FILE(file_idx));
        self.recording.push(Operation::CREATE {
            path: self.resolve_dir_path(&parent).join(name),
            mode,
        });
        self.nodes_created += 1;
        Ok(file_idx)
    }

    pub fn hardlink(&mut self, old_path: PathName, new_path: PathName) -> Result<FileIndex> {
        let old_file = self.resolve_file(old_path.clone())?;
        let (parent_path, name) = new_path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        if self.name_exists(&parent, &name) {
            return Err(ExecutorError::NameAlreadyExists(
                self.resolve_dir_path(&parent).join(name),
            ));
        }
        let file = self.file_mut(&old_file);
        file.parents.insert(parent.to_owned());
        let parent_dir = self.dir_mut(&parent);
        parent_dir
            .children
            .insert(name.clone(), Node::FILE(old_file.to_owned()));
        self.recording
            .push(Operation::HARDLINK { old_path, new_path });
        self.nodes_created += 1;
        Ok(old_file.to_owned())
    }

    pub fn rename(&mut self, old_path: PathName, new_path: PathName) -> Result<FileIndex> {
        let old_file = self.resolve_file(old_path.clone())?;
        let (parent_path, name) = old_path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        let parent_dir = self.dir_mut(&parent);
        parent_dir.children.remove(&name);
        let (parent_path, name) = new_path.split();
        let parent = self.resolve_dir(parent_path.to_owned())?;
        let parent_dir = self.dir_mut(&parent);
        parent_dir
            .children
            .insert(name.clone(), Node::FILE(old_file.to_owned()));
        self.recording
            .push(Operation::RENAME { old_path, new_path });
        self.nodes_created += 1;
        Ok(old_file.to_owned())
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

    pub fn resolve_file_path(&self, file_idx: &FileIndex) -> Vec<PathName> {
        let mut paths = vec![];
        let file = self.file(file_idx);
        for dir in file.parents.iter() {
            self.dir(dir)
                .children
                .iter()
                .filter(|(_, node)| **node == Node::FILE(file_idx.to_owned()))
                .for_each(|(name, _)| paths.push(self.resolve_dir_path(dir).join(name.clone())));
        }
        paths.sort();
        paths
    }

    pub fn resolve_dir_path(&self, dir_idx: &DirIndex) -> PathName {
        let mut segments: Vec<String> = vec![];
        let mut next = dir_idx.to_owned();
        loop {
            let dir = self.dir(&next);
            match dir.parent {
                Some(parent) => {
                    let parent_dir = self.dir(&parent);
                    let (name, _) = parent_dir
                        .children
                        .iter()
                        .find(|(_, node)| **node == Node::DIR(next))
                        .unwrap();
                    segments.push(name.clone());
                    next = parent;
                }
                None => break,
            }
        }
        segments.reverse();
        ("/".to_owned() + segments.join("/").as_str()).into()
    }

    pub fn resolve_path(&self, node: &Node) -> Vec<PathName> {
        match node {
            Node::FILE(file) => self.resolve_file_path(file),
            Node::DIR(dir) => vec![self.resolve_dir_path(dir)],
        }
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
        assert_eq!(1, exec.nodes_created);
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
        assert_eq!(1, exec.nodes_created);
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
        assert_eq!(2, exec.nodes_created);
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

        let mut parents = HashSet::new();
        parents.insert(AbstractExecutor::root_index());
        parents.insert(bar);
        assert_eq!(parents, exec.file(&foo).parents);
        assert_eq!(parents, exec.file(&boo).parents);

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
        assert_eq!(3, exec.nodes_created);
        test_replay(exec.recording);
    }

    #[test]
    fn test_remove_hardlink() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.create("/foo".into(), vec![]).unwrap();
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

        let mut parents = HashSet::new();
        parents.insert(AbstractExecutor::root_index());
        assert_eq!(parents, exec.file(&foo).parents);

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
        assert_eq!(2, exec.nodes_created);
        test_replay(exec.recording);
    }

    #[test]
    fn test_remove_hardlink_dir() {
        let mut exec = AbstractExecutor::new();
        let zero = exec.create("/0".into(), vec![]).unwrap();
        exec.mkdir("/1".into(), vec![]).unwrap();
        exec.mkdir("/1/2".into(), vec![]).unwrap();
        exec.hardlink("/0".into(), "/1/2/3".into()).unwrap();
        exec.remove("/1".into()).unwrap();
        assert_eq!(vec![PathName::from("/0")], exec.resolve_file_path(&zero));
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
        assert_eq!(2, exec.nodes_created);
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
        assert_eq!(2, exec.nodes_created);
        test_replay(exec.recording);
    }

    #[test]
    fn test_resolve_path() {
        let mut exec = AbstractExecutor::new();
        let foo = exec.mkdir("/foo".into(), vec![]).unwrap();
        let bar = exec.mkdir("/foo/bar".into(), vec![]).unwrap();
        let boo = exec.create("/foo/bar/boo".into(), vec![]).unwrap();
        exec.hardlink("/foo/bar/boo".into(), "/zoo".into()).unwrap();
        exec.hardlink("/foo/bar/boo".into(), "/foo/bar/moo".into())
            .unwrap();

        assert_eq!(
            vec![PathName::from("/foo")],
            exec.resolve_path(&Node::DIR(foo))
        );
        assert_eq!(
            vec![PathName::from("/foo/bar")],
            exec.resolve_path(&Node::DIR(bar))
        );
        assert_eq!(
            vec![
                PathName::from("/foo/bar/boo"),
                PathName::from("/foo/bar/moo"),
                PathName::from("/zoo")
            ],
            exec.resolve_path(&Node::FILE(boo))
        );
        assert_eq!(5, exec.nodes_created);
        test_replay(exec.recording);
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
        assert_eq!(3, exec.nodes_created);
        test_replay(exec.recording);
    }

    fn test_replay(workload: Workload) {
        let mut exec = AbstractExecutor::new();
        exec.replay(&workload).unwrap();
        assert_eq!(workload, exec.recording);
    }
}
