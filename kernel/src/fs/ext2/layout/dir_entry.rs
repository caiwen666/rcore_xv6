use zerocopy::{FromBytes, IntoBytes, KnownLayout};

/// 目录项头部
#[derive(Debug, Clone, IntoBytes, FromBytes, KnownLayout)]
#[repr(C)]
pub struct DirEntryHeader {
    /// 指向的 inode 编号
    pub inode: u32,
    /// 本条目录项的总长度（含头部、文件名与 padding）
    pub rec_len: u16,
    /// 文件名长度
    pub name_len: u8,
    /// 文件类型
    _file_type: u8,
}
