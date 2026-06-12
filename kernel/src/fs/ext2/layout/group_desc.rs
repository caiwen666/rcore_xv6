use zerocopy::{FromBytes, IntoBytes, KnownLayout};

#[derive(Debug, Clone, IntoBytes, FromBytes, KnownLayout)]
#[repr(C)]
pub struct GroupDescriptor {
    /// 块位图起始块号
    pub block_bitmap: u32,
    /// inode 位图起始块号
    pub inode_bitmap: u32,
    /// inode 表起始块号
    pub inode_table: u32,
    /// 空闲块数量
    pub free_blocks_count: u16,
    /// 空闲 inode 数量
    pub free_inodes_count: u16,
    /// 目录数量
    pub dirs_count: u16,
    _padding: [u8; 14],
}
