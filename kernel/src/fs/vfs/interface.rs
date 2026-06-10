use alloc::{string::String, sync::Arc, vec::Vec};

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
    /// 更改文件大小
    ///
    /// VFS 框架保证，调用该函数时，当前 inode 的类型一定是文件类型
    ///
    /// **可能会堵塞**
    fn resize(&self, new_size: usize);

    /// 在 inode 的指定偏移量开始，读取指定大小的数据
    ///
    /// VFS 框架保证，读取的数据的范围在文件大小范围以内
    ///
    /// **可能会堵塞**
    fn read_at(&self, offset: usize, buf: &mut [u8]);

    /// 在 inode 的指定偏移量开始，写入指定大小的数据
    ///
    /// VFS 框架保证，读取的数据的范围在文件大小范围以内
    ///
    /// **会堵塞**
    fn write_at(&self, offset: usize, buf: &[u8]);

    /// 在当前 inode 下寻找指定名称的文件。
    ///
    /// VFS 框架保证调用时，当前 inode 一定是目录类型
    ///
    /// **可能会阻塞**
    ///
    /// # Returns
    ///
    /// - 如果存在，则返回对应文件的 inode 编号，该编号会影响 VFS 的 inode 缓存
    /// - 如果不存在则返回 None
    fn find(&self, name: &str) -> Option<u64>;

    /// 获取父目录
    ///
    /// # Returns
    ///
    /// - 如果存在，则返回父目录的 inode 编号，该编号会影响 VFS 的 inode 缓存
    /// - 如果当前已经为根目录了，则返回 None
    fn parent(&self) -> Option<u64>;

    /// 获取文件的元数据
    ///
    /// VFS 框架不会缓存 metadata，该函数会被频繁调用
    fn metadata(&self) -> Metadata;

    /// 列出当前目录下的所有文件名称，VFS 框架保证调用该函数时，当前 inode 的类型一定是目录类型
    ///
    /// **会阻塞**
    fn list(&self) -> Vec<String>;

    /// 获取 inode 的编号，该编号会影响 VFS 框架对 inode 的缓存
    fn id(&self) -> u64;
}

pub trait FileSystem: Send + Sync {
    /// 获取文件系统的根 inode 的编号，该编号会影响 VFS 框架对 inode 的缓存
    fn root_inode(&self) -> u64;
    /// 生成一个 inode 实例，VFS 框架有 inode 缓存机制，该函数不会调用太频繁
    fn get_inode(&self, id: u64) -> Arc<dyn IndexNode>;
    /// 文件系统名称
    #[expect(unused)]
    fn name(&self) -> &'static str;
}
