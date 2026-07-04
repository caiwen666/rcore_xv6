use crate::fs::vfs::{VirtualFileSystem, VirtualIndexNode, interface::FileType};
use alloc::{string::String, sync::Arc};

impl VirtualIndexNode {
    /// 寻找当前目录下指定名称的文件的 inode，会自动处理挂载点
    ///
    /// # Panics
    ///
    /// - 如果当前 inode 的类型不是目录，则 panic
    pub fn find(&self, name: &str) -> Option<VirtualIndexNode> {
        if self.metadata().file_type != FileType::Directory {
            panic!("current inode is not a directory");
        }
        self.find_with_cache(name).map(|inode| {
            let fs = self.fs();
            // 走到挂载点了，跳过去
            // 这里不能把 mount_fs 写到一行上去，不然 mountpoints.lock() 的 guard 生命周期会延长到
            // 整合 if 块。
            let mount_fs = fs.mountpoints.lock().get(&inode.id).cloned();
            if let Some(mount_fs) = mount_fs {
                let root_inode_id = mount_fs.inner_fs.root_inode();
                mount_fs.get_inode_with_cache(root_inode_id)
            } else {
                inode
            }
        })
    }

    /// 获取当前文件的父目录的 inode，会自动处理挂载点
    ///
    /// # Returns
    ///
    /// 如果已经是根目录了，则返回 None
    ///
    /// # Panics
    ///
    /// - 如果当前 inode 的类型不是目录，则 panic
    pub fn parent(&self) -> Option<VirtualIndexNode> {
        if self.metadata().file_type != FileType::Directory {
            panic!("current inode is not a directory");
        }
        let fs = self.fs();
        let inode = self.inode();
        if let Some(parent_inode_id) = inode.parent() {
            Some(fs.get_inode_with_cache(parent_inode_id))
        } else {
            let mount_parent = fs.self_mountpoints.lock().as_ref().cloned();
            mount_parent.and_then(|inode| inode.parent())
        }
    }

    /// 获取当前目录的名称
    ///
    /// # Returns
    ///
    /// - 如果当前目录是根目录，则返回 None
    /// - 如果当前目录被从父目录中删除，则返回 None
    ///
    /// # Panics
    ///
    /// 如果当前 inode 类型不是目录，则 panic
    pub fn dir_name(&self) -> Option<String> {
        assert!(
            self.metadata().file_type == FileType::Directory,
            "current inode is not a directory"
        );
        let fs = self.fs();
        let inode = self.inode();
        if let Some(parent_inode_id) = inode.parent() {
            let parent_inode = fs.get_inode_with_cache(parent_inode_id);
            // TODO 需要优化
            let list = parent_inode.list();
            list.iter()
                .find(|(_, id)| *id == self.id)
                .map(|(name, _)| name.clone())
        } else {
            let mount_parent = fs.self_mountpoints.lock().as_ref().cloned();
            mount_parent.and_then(|inode| inode.dir_name())
        }
    }

    /// 将当前目录挂载为另一个文件系统的根目录
    ///
    /// # Panics
    ///
    /// - 如果当前 inode 的类型不是目录，则 panic
    /// - 如果当前 inode 已经是挂载点，则 panic
    /// - 如果当前 inode 是当前文件系统的根目录，则 panic
    #[expect(unused)]
    pub fn mount(&self, mut target_fs: Arc<VirtualFileSystem>) {
        if self.metadata().file_type != FileType::Directory {
            panic!("current inode is not a directory");
        }
        let fs = self.fs();
        let mut mountpoints = fs.mountpoints.lock();
        if mountpoints.contains_key(&self.id) {
            panic!("current inode is already a mount point");
        }
        if self.id == fs.inner_fs.root_inode() {
            panic!("current inode is the root inode of the current file system");
        }
        target_fs.self_mountpoints.lock().insert(self.clone());
        mountpoints.insert(self.id, target_fs);
    }
}
