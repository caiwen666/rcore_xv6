use crate::utils::BlockIterator;
use alloc::vec;
use alloc::{string::String, sync::Arc, vec::Vec};

pub trait BlockDevice: Send + Sync + 'static {
    /// 块设备的块大小
    fn block_size(&self) -> usize;
    /// 读取一个块
    ///
    /// # Parameters
    ///
    /// - `block_id`：块的编号，从 0 开始
    /// - `buf`：缓冲区，大小需要与 block_size 一致
    ///
    /// # Preconditions
    ///
    /// `buf` 应为一个物理空间上连续的内存区域，并且内存的物理地址和虚拟地址相等
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    /// 写入一个块
    ///
    /// # Parameters
    ///
    /// - `block_id`：块的编号，从 0 开始
    /// - `buf`：缓冲区，大小需要与 block_size 一致
    ///
    /// # Preconditions
    ///
    /// `buf` 应为一个物理空间上连续的内存区域，并且内存的物理地址和虚拟地址相等
    #[expect(unused)]
    fn write_block(&self, block_id: usize, buf: &[u8]);
    /// 从 `offset` 处开始，读取数据到 `buf`
    ///
    /// # Parameters
    ///
    /// - `offset`：起始偏移量
    /// - `buf`：缓冲区，大小需要与 `size` 一致
    ///
    /// # Preconditions
    ///
    /// 如果超出了块设备的范围，则出现未定义行为
    fn read_at(&self, offset: usize, buf: &mut [u8]) {
        let block_size = self.block_size();
        let mut block_buf = vec![0u8; block_size];
        let mut pos = 0;
        for block in BlockIterator::new(block_size, offset, buf.len()) {
            self.read_block(block.block_id(), block_buf.as_mut_slice());
            buf[pos..pos + block.size()]
                .copy_from_slice(&block_buf[block.offset()..block.offset() + block.size()]);
            pos += block.size();
        }
    }
    /// 从 `offset` 处开始，写入数据到 `buf`
    ///
    /// # Parameters
    ///
    /// - `offset`：起始偏移量
    /// - `buf`：缓冲区
    ///
    /// # Preconditions
    ///
    /// 如果超出了块设备的范围，则出现未定义行为
    #[expect(unused)]
    fn write_at(&self, offset: usize, buf: &[u8]) {
        let block_size = self.block_size();
        let mut block_buf = vec![0u8; block_size];
        let mut pos = 0;
        for block in BlockIterator::new(block_size, offset, buf.len()) {
            // 由于是按整块写入的，需要先读取原来的数据
            self.read_block(block.block_id(), block_buf.as_mut_slice());
            block_buf[block.offset()..block.offset() + block.size()]
                .copy_from_slice(&buf[pos..pos + block.size()]);
            self.write_block(block.block_id(), block_buf.as_mut_slice());
            pos += block.size();
        }
    }
}

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
    /// VFS 框架保证调用时，当前 inode 一定是目录类型
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
    ///
    /// # Returns
    ///
    /// 返回一个二元组列表，第一个元素是文件名称，第二个元素是文件的 inode 编号
    fn list(&self) -> Vec<(String, u64)>;

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
