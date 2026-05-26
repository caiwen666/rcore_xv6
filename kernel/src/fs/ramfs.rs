use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};

use crate::{
    fs::vfs::{FileSystem, IndexNode},
    sync::spin::SpinMutex,
};

pub struct RamFS {
    root: Arc<RamFSInode>,
}

impl RamFS {
    pub fn new() -> Self {
        Self {
            root: Arc::new_cyclic(|weak| RamFSInode {
                name: "root".to_string(),
                parent: None,
                inner: SpinMutex::new(
                    RamFSInodeType::Directory(BTreeMap::new()),
                    "ramfs_inode_inner",
                ),
                self_weak: weak.clone(),
            }),
        }
    }

    pub fn root(&self) -> Arc<RamFSInode> {
        self.root.clone()
    }
}

pub struct RamFSInode {
    name: String,
    parent: Option<Weak<RamFSInode>>,
    inner: SpinMutex<RamFSInodeType>,
    self_weak: Weak<RamFSInode>,
}

impl RamFSInode {
    /// 在当前节点下创建一个目录
    ///
    /// # Returns
    ///
    /// 返回新创建的目录
    ///
    /// # Panics
    ///
    /// - 如果当前节点不是目录，则 panic
    /// - 如果已有同名的文件或目录，则 panic
    pub fn push_directory(&self, name: String) -> Arc<RamFSInode> {
        let mut inner = self.inner.lock();
        if let RamFSInodeType::Directory(ref mut map) = *inner {
            if map.contains_key(&name) {
                panic!("directory already exists");
            }
            let directory = Arc::new_cyclic(|weak| RamFSInode {
                name: name.clone(),
                parent: Some(self.self_weak.clone()),
                inner: SpinMutex::new(
                    RamFSInodeType::Directory(BTreeMap::new()),
                    "ramfs_inode_inner",
                ),
                self_weak: weak.clone(),
            });
            map.insert(name, directory.clone());
            directory
        } else {
            panic!("not a directory");
        }
    }

    /// 在当前节点下创建一个文件
    ///
    /// # Returns
    ///
    /// 返回新创建的文件
    ///
    /// # Panics
    ///
    /// - 如果当前节点不是目录，则 panic
    /// - 如果已有同名的文件或目录，则 panic
    pub fn push_file(&self, name: String, content: Vec<u8>) -> Arc<RamFSInode> {
        let mut inner = self.inner.lock();
        if let RamFSInodeType::Directory(ref mut map) = *inner {
            if map.contains_key(&name) {
                panic!("file already exists");
            }
            let file = Arc::new_cyclic(|weak| RamFSInode {
                name: name.clone(),
                parent: Some(self.self_weak.clone()),
                inner: SpinMutex::new(RamFSInodeType::File(content), "ramfs_inode_inner"),
                self_weak: weak.clone(),
            });
            map.insert(name, file.clone());
            file
        } else {
            panic!("not a directory");
        }
    }
}

pub enum RamFSInodeType {
    File(Vec<u8>),
    Directory(BTreeMap<String, Arc<RamFSInode>>),
}

impl IndexNode for RamFSInode {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let inner = self.inner.lock();
        if let RamFSInodeType::File(ref content) = *inner {
            let arr = content.as_slice();
            let len = buf.len().min(arr.len() - offset);
            buf[..len].copy_from_slice(&arr[offset..offset + len]);
            len
        } else {
            panic!("not a file");
        }
    }

    #[expect(clippy::map_clone)]
    fn find(&self, name: &str) -> Option<Arc<dyn IndexNode>> {
        let inner = self.inner.lock();
        if let RamFSInodeType::Directory(ref map) = *inner {
            map.get(name)
                .map(|inode| -> Arc<dyn IndexNode> { inode.clone() })
        } else {
            panic!("not a directory");
        }
    }

    fn parent(&self) -> Option<Arc<dyn IndexNode>> {
        self.parent
            .as_ref()
            .map(|parent| -> Arc<dyn IndexNode> { parent.upgrade().unwrap() })
    }

    /// 对于目录，返回的元数据大小为 0
    fn metadata(&self) -> super::vfs::Metadata {
        let inner = self.inner.lock();
        if let RamFSInodeType::File(ref content) = *inner {
            super::vfs::Metadata {
                name: self.name.clone(),
                file_type: super::vfs::FileType::File,
                size: content.len(),
            }
        } else {
            super::vfs::Metadata {
                name: self.name.clone(),
                file_type: super::vfs::FileType::Directory,
                size: 0,
            }
        }
    }

    fn list(&self) -> Vec<String> {
        let inner = self.inner.lock();
        if let RamFSInodeType::Directory(ref map) = *inner {
            map.keys().cloned().collect()
        } else {
            panic!("not a directory");
        }
    }
}

impl FileSystem for RamFS {
    fn root_inode(&self) -> Arc<dyn IndexNode> {
        self.root.clone()
    }

    fn name(&self) -> &'static str {
        "ramfs"
    }
}
