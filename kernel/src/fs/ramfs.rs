use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};

use crate::{
    fs::vfs::{
        FileSystem, IndexNode,
        interface::{DirectoryEntry, FileType, Metadata},
    },
    sync::spin::SpinMutex,
};

pub struct RamFS {
    inode_map: BTreeMap<u64, Arc<RamFSInode>>,
    unused_id: u64,
}

impl RamFS {
    pub fn new() -> (Arc<SpinMutex<Self>>, Arc<RamFSInode>) {
        let fs = Arc::new_cyclic(|fs_weak| {
            let mut inode_map: BTreeMap<u64, Arc<RamFSInode>> = BTreeMap::new();
            inode_map.insert(
                0,
                Arc::new_cyclic(|inode_weak| RamFSInode {
                    parent: None,
                    inner: SpinMutex::new(
                        RamFSInodeType::Directory(BTreeMap::new()),
                        "ramfs_inode_inner",
                    ),
                    self_weak: inode_weak.clone(),
                    fs: fs_weak.clone(),
                    id: 0,
                }),
            );
            SpinMutex::new(
                Self {
                    inode_map,
                    unused_id: 1,
                },
                "ramfs",
            )
        });
        (fs.clone(), fs.lock().inode_map.get(&0).cloned().unwrap())
    }

    pub fn alloc_id(&mut self) -> u64 {
        let res = self.unused_id;
        self.unused_id += 1;
        res
    }
}

pub struct RamFSInode {
    parent: Option<Weak<RamFSInode>>,
    inner: SpinMutex<RamFSInodeType>,
    self_weak: Weak<RamFSInode>,
    fs: Weak<SpinMutex<RamFS>>,
    id: u64,
}

pub enum RamFSInodeType {
    File(Vec<u8>),
    // BTreeMap 的 key 中，第一个元素表示目录项的名称，第二个元素表示目录项是否为文件
    Directory(BTreeMap<(String, bool), u64>),
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
    /// - 如果已有同名的文件或目录，则 panic
    pub fn push_directory(&self, name: String) -> Arc<RamFSInode> {
        let mut inner = self.inner.lock();
        let RamFSInodeType::Directory(ref mut map) = *inner else {
            // VFS 框架已经保证不可能出现这种情况
            unreachable!();
        };
        if map.contains_key(&(name.clone(), false)) {
            panic!("directory already exists");
        }
        let fs = self.fs.upgrade().unwrap();
        let mut fs = fs.lock();
        let id = fs.alloc_id();
        let directory = Arc::new_cyclic(|weak| RamFSInode {
            parent: Some(self.self_weak.clone()),
            inner: SpinMutex::new(
                RamFSInodeType::Directory(BTreeMap::new()),
                "ramfs_inode_inner",
            ),
            self_weak: weak.clone(),
            fs: self.fs.clone(),
            id,
        });
        fs.inode_map.insert(id, directory.clone());
        map.insert((name, false), id);
        directory
    }

    /// 在当前节点下创建一个文件
    ///
    /// # Returns
    ///
    /// 返回新创建的文件
    ///
    /// # Panics
    ///
    /// - 如果已有同名的文件或目录，则 panic
    pub fn push_file(&self, name: String, content: Vec<u8>) -> Arc<RamFSInode> {
        let mut inner = self.inner.lock();
        let RamFSInodeType::Directory(ref mut map) = *inner else {
            // VFS 框架已经保证不可能出现这种情况
            unreachable!();
        };
        if map.contains_key(&(name.clone(), true)) {
            panic!("file already exists");
        }
        let fs = self.fs.upgrade().unwrap();
        let mut fs = fs.lock();
        let id = fs.alloc_id();
        let file = Arc::new_cyclic(|weak| RamFSInode {
            parent: Some(self.self_weak.clone()),
            inner: SpinMutex::new(RamFSInodeType::File(content), "ramfs_inode_inner"),
            self_weak: weak.clone(),
            fs: self.fs.clone(),
            id,
        });
        fs.inode_map.insert(id, file.clone());
        map.insert((name, true), id);
        file
    }
}

impl IndexNode for RamFSInode {
    fn read_at(&self, offset: usize, buf: &mut [u8]) {
        let inner = self.inner.lock();
        if let RamFSInodeType::File(ref content) = *inner {
            let arr = content.as_slice();
            let len = buf.len().min(arr.len() - offset);
            buf[..len].copy_from_slice(&arr[offset..offset + len]);
        } else {
            panic!("not a file");
        }
    }

    fn find(&self, name: &str) -> Option<u64> {
        let mut inner = self.inner.lock();
        let RamFSInodeType::Directory(ref mut map) = *inner else {
            // VFS 框架已经保证不可能出现这种情况
            unreachable!();
        };
        // TODO 同名文件/目录支持
        map.get(&(name.to_string(), false))
            .cloned()
            .or_else(|| map.get(&(name.to_string(), true)).cloned())
    }

    fn parent(&self) -> Option<u64> {
        self.parent
            .as_ref()
            .map(|parent| parent.upgrade().unwrap().id())
    }

    /// 对于目录，返回的元数据大小为 0
    fn metadata(&self) -> Metadata {
        let inner = self.inner.lock();
        match *inner {
            RamFSInodeType::File(ref content) => Metadata {
                file_type: FileType::File,
                size: content.len(),
            },
            RamFSInodeType::Directory(ref map) => Metadata {
                file_type: FileType::Directory,
                size: map.len(),
            },
        }
    }

    fn read_dir(&self, offset_cookie: u64) -> Option<DirectoryEntry> {
        let inner = self.inner.lock();
        let RamFSInodeType::Directory(ref map) = *inner else {
            // VFS 框架已经保证不可能出现这种情况
            unreachable!();
        };
        // ramfs 的目录项一般不多，所以直接遍历
        let ((name, is_file), &inode_id) = map.iter().nth(offset_cookie as usize)?;
        Some(DirectoryEntry {
            name: name.clone(),
            offset_cookie: offset_cookie + 1,
            inode: inode_id,
            file_type: if *is_file {
                FileType::File
            } else {
                FileType::Directory
            },
        })
    }

    fn resize(&self, _new_size: usize) {
        unimplemented!()
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) {
        unimplemented!()
    }

    fn id(&self) -> u64 {
        self.id
    }
}

impl FileSystem for SpinMutex<RamFS> {
    fn root_inode(&self) -> u64 {
        0
    }

    fn name(&self) -> &'static str {
        "ramfs"
    }

    fn get_inode(&self, id: u64) -> Arc<dyn IndexNode> {
        let fs = self.lock();
        fs.inode_map.get(&id).cloned().unwrap()
    }
}
