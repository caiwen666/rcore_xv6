//! 虚拟文件系统框架
//!
//! 框架的主要目的是统一各个文件系统的接口，同时还做到如下几点：
//! 1. 提供文件系统挂载功能，可以把一个文件系统挂载到其他文件系统中，框架自动处理跨文件系统跳转
//! 2. 提供整套的 Inode 缓存机制
//! 3. 提供 PageCache 缓存机制

mod inode_cache;
pub mod interface;
mod mount;
mod page_cache;

use core::{ops::Deref, ptr::NonNull};

use crate::{
    fs::{
        ROOT_FS,
        vfs::{
            interface::{FileType, Metadata},
            page_cache::PageCache,
        },
    },
    sync::{condvar::Condvar, spin::SpinMutex},
};
use alloc::{
    collections::btree_map::BTreeMap,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};

pub(super) use interface::{FileSystem, IndexNode};

/// 虚拟文件系统
pub struct VirtualFileSystem {
    inner_fs: Arc<dyn FileSystem>,
    /// 当前文件系统下的挂载点，以实现文件系统的递归挂载
    mountpoints: SpinMutex<BTreeMap<u64, Arc<VirtualFileSystem>>>,
    /// 当前被挂载的文件系统对应的挂载点的 Inode，
    /// 记录这个以实现从被挂载文件系统向父目录跳跃时跨文件系统
    ///
    /// 如果当前文件系统是最根上的文件系统，没有被挂载到其他文件
    /// 系统中的话，这里就为 None
    self_mountpoints: Option<VirtualIndexNode>,
    self_weak: Weak<Self>,
    inode_cache: SpinMutex<BTreeMap<u64, VirtualIndexNode>>,
}

impl VirtualFileSystem {
    /// 使用 VFS 接管指定文件系统
    pub fn new(fs: Arc<dyn FileSystem>) -> Arc<Self> {
        Arc::new_cyclic(|weak| Self {
            inner_fs: fs,
            mountpoints: SpinMutex::new(BTreeMap::new(), "vfs_mountpoints"),
            self_mountpoints: None,
            self_weak: weak.clone(),
            inode_cache: SpinMutex::new(BTreeMap::new(), "vfs_inode_cache"),
        })
    }

    pub fn root(&self) -> VirtualIndexNode {
        let inode = self.inner_fs.root_inode();
        self.get_inode_with_cache(inode)
    }
}

pub struct VirtualIndexNode {
    ptr: NonNull<VirtualIndexNodeInner>,
    fs: Weak<VirtualFileSystem>,
    weak: bool,
}

unsafe impl Send for VirtualIndexNode {}
unsafe impl Sync for VirtualIndexNode {}
impl VirtualIndexNode {
    pub fn fs(&self) -> Arc<VirtualFileSystem> {
        self.fs
            .upgrade()
            .expect("Failed to upgrade weak reference to strong reference")
    }

    pub fn metadata(&self) -> Metadata {
        self.inner_locked.lock().inode().metadata()
    }

    /// 列出当前目录下的所有文件名称
    ///
    /// # Panics
    ///
    /// - 如果当前 inode 的类型不是目录，则 panic
    pub fn list(&self) -> Vec<String> {
        let inner_locked = self.inner_locked.lock();
        let inode = inner_locked.inode();
        if inode.metadata().file_type != FileType::Directory {
            panic!("current inode is not a directory");
        }
        inode.list()
    }
}

impl Deref for VirtualIndexNode {
    type Target = VirtualIndexNodeInner;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

pub struct VirtualIndexNodeInner {
    /// 如果当前 inode 正在初始化中，那么当前线程从 inner_locked 中
    /// 拿 inode 会得到 None，此时需要带 inner_locked 在 condvar 上面
    /// 等待
    condvar: Condvar,
    inner_locked: SpinMutex<VirtualIndexNodeInnerLocked>,
    page_cache: PageCache,
    id: u64,
}

pub struct VirtualIndexNodeInnerLocked {
    /// 如果当前 inode 正在初始化中，这里就是 None
    inode: Option<Arc<dyn IndexNode>>,
    ref_count: usize,
    /// 是否正在销毁
    destroying: bool,
}

impl VirtualIndexNodeInnerLocked {
    pub fn inode(&self) -> Arc<dyn IndexNode> {
        self.inode.clone().expect("Failed to get inode")
    }
}

/// 以 `base` 为基准，查找 `path` 对应的 inode
pub fn lookup(base: VirtualIndexNode, path: &str) -> Option<VirtualIndexNode> {
    let mut current = if path.starts_with('/') {
        ROOT_FS.root()
    } else {
        base
    };
    for component in path.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if component == ".." {
            current = current.parent()?;
            continue;
        }
        current = current.find(component)?;
    }
    Some(current)
}
