use crate::{
    fs::vfs::{FileSystem, IndexNode},
    sync::spin::SpinMutex,
    utils::ArcPtr,
};
use alloc::{
    collections::btree_map::BTreeMap,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};

/// 所有被挂载的文件系统都被这个包裹，以实现文件系统的递归挂载
pub struct MountFS {
    inner: Arc<dyn FileSystem>,
    /// 当前文件系统下的挂载点，以实现文件系统的递归挂载
    mountpoints: SpinMutex<BTreeMap<ArcPtr<dyn IndexNode>, Arc<MountFS>>>,
    /// 当前被挂载的文件系统对应的挂载点的 Inode，
    /// 记录这个以实现从被挂载文件系统向父目录跳跃时跨文件系统
    ///
    /// 如果当前文件系统是最根上的文件系统，没有被挂载到其他文件
    /// 系统中的话，这里就为 None
    self_mountpoints: Option<Arc<MountFSInode>>,
    self_weak: Weak<Self>,
}

impl MountFS {
    /// 创建一个根文件系统
    pub fn new_root(fs: Arc<dyn FileSystem>) -> Arc<Self> {
        Arc::new_cyclic(|weak| Self {
            inner: fs,
            mountpoints: SpinMutex::new(BTreeMap::new(), "vfs_mountpoints"),
            self_mountpoints: None,
            self_weak: weak.clone(),
        })
    }

    pub fn root(&self) -> Arc<MountFSInode> {
        Arc::new(MountFSInode {
            inner: self.inner.root_inode(),
            mount_fs: self.self_weak.clone(),
        })
    }
}

pub struct MountFSInode {
    inner: Arc<dyn IndexNode>,
    mount_fs: Weak<MountFS>,
}

impl IndexNode for MountFSInode {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.inner.read_at(offset, buf)
    }

    fn find(&self, name: &str) -> Option<Arc<dyn IndexNode>> {
        self.inner.find(name).map(|inode| -> Arc<dyn IndexNode> {
            let arc_ptr = ArcPtr::new(inode.clone());
            let mount_fs = self.mount_fs.upgrade().unwrap();
            if let Some(mount_fs) = mount_fs.mountpoints.lock().get(&arc_ptr) {
                Arc::new(MountFSInode {
                    inner: mount_fs.inner.root_inode(),
                    mount_fs: Arc::downgrade(mount_fs),
                })
            } else {
                Arc::new(MountFSInode {
                    inner: inode,
                    mount_fs: self.mount_fs.clone(),
                })
            }
        })
    }

    fn parent(&self) -> Option<Arc<dyn IndexNode>> {
        let mount_fs = self.mount_fs.upgrade().unwrap();
        if Arc::ptr_eq(&self.inner, &mount_fs.inner.root_inode()) {
            // 需要跨越文件系统
            match &mount_fs.self_mountpoints {
                None => None,
                Some(self_mountpoints) => self_mountpoints.parent(),
            }
        } else {
            Some(Arc::new(MountFSInode {
                inner: self.inner.parent().unwrap(),
                mount_fs: self.mount_fs.clone(),
            }))
        }
    }

    fn metadata(&self) -> super::Metadata {
        self.inner.metadata()
    }

    fn mount(&self, fs: Arc<dyn FileSystem>) {
        if self.inner.metadata().file_type != super::FileType::Directory {
            panic!("current inode is not a directory");
        }
        let mount_fs = self.mount_fs.upgrade().unwrap();
        if mount_fs
            .mountpoints
            .lock()
            .contains_key(&ArcPtr::new(self.inner.clone()))
        {
            panic!("current inode is already a mount point");
        }
        if Arc::ptr_eq(&self.inner, &mount_fs.inner.root_inode()) {
            panic!("current inode is the root inode of the current file system");
        }
        let new_mount_fs = Arc::new_cyclic(|weak| MountFS {
            inner: fs,
            mountpoints: SpinMutex::new(BTreeMap::new(), "vfs_mountpoints"),
            self_mountpoints: Some(Arc::new(MountFSInode {
                inner: self.inner.clone(),
                mount_fs: self.mount_fs.clone(),
            })),
            self_weak: weak.clone(),
        });
        mount_fs
            .mountpoints
            .lock()
            .insert(ArcPtr::new(self.inner.clone()), new_mount_fs);
    }

    fn list(&self) -> Vec<String> {
        self.inner.list()
    }
}
