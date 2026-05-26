pub mod mount;

use alloc::{string::String, sync::Arc, vec::Vec};

use crate::fs::ROOT_FS;

/// 文件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
}

/// 文件的元数据
#[derive(Debug, Clone)]
pub struct Metadata {
    pub file_type: FileType,
    /// 文件大小。单位：字节
    pub size: usize,
    /// 文件名称
    #[expect(unused)]
    pub name: String,
}

pub trait IndexNode: Send + Sync {
    /// 在 inode 的指定偏移量开始，读取指定大小的数据
    ///
    /// 如果读取到的数据长度小于 `buf` 的长度，则 `buf`
    /// 剩余部分不会被修改。
    ///
    /// # Returns
    ///
    /// 返回成功读取的字节数
    ///
    /// # Panics
    ///
    /// - 如果当前 inode 不是文件，则 panic
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize;

    /// 在当前 inode 下寻找指定名称的文件
    ///
    /// # Returns
    ///
    /// - 对于同一文件实体，返回的 Arc 应该要指向同一 IndexNode 实例，
    ///   否则会导致未定义行为。
    /// - 如果不存在则返回 None
    ///
    /// # Panics
    ///
    /// - 如果当前 inode 不是目录，则 panic
    fn find(&self, name: &str) -> Option<Arc<dyn IndexNode>>;

    /// 获取父目录
    ///
    /// # Returns
    ///
    /// - 对于同一文件实体，返回的 Arc 应该要指向同一 IndexNode 实例，
    ///   否则会导致未定义行为。
    /// - 如果当前已经为根目录了，则返回 None
    fn parent(&self) -> Option<Arc<dyn IndexNode>>;

    /// 获取文件的元数据
    fn metadata(&self) -> Metadata;

    /// 将当前 inode 作为挂载点，挂载文件系统
    ///
    /// # Parameters
    ///
    /// - `fs`: 要挂载的文件系统
    ///
    ///
    /// # Panics
    ///
    /// - 如果当前文件系统没有实现挂载功能，则 panic
    /// - 如果当前 inode 已经是一个挂载点，则 panic
    /// - 如果当前 inode 是当前文件系统的根目录则 panic
    /// - 如果当前 inode 不是目录，则 panic
    ///
    /// # Notes
    ///
    /// 这个专门给 MountFS 实现的，其他的文件系统不应该实现这个
    #[expect(unused)]
    fn mount(&self, _fs: Arc<dyn FileSystem>) {
        unimplemented!()
    }

    /// 列出当前目录下的所有文件名称
    ///
    /// # Panics
    ///
    /// 如果当前 inode 不是目录，则 panic
    fn list(&self) -> Vec<String>;
}

pub trait FileSystem: Send + Sync {
    /// 获取文件系统的根 inode
    fn root_inode(&self) -> Arc<dyn IndexNode>;
    /// 文件系统名称
    #[expect(unused)]
    fn name(&self) -> &'static str;
}

/// 以 `base` 为基准，查找 `path` 对应的 inode
pub fn lookup(base: Arc<dyn IndexNode>, path: &str) -> Option<Arc<dyn IndexNode>> {
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
