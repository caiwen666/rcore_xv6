use zerocopy::{FromBytes, IntoBytes};

#[derive(Debug, Clone, IntoBytes, FromBytes)]
#[repr(C)]
pub struct Inode {
    /// 文件类型和访问权限位
    pub mode: u16,
    /// 所有者的用户 ID
    _uid: u16,
    /// 文件大小
    pub size: u32,
    /// 最后一次访问时间
    ///
    /// Unix 时间戳
    _atime: u32,
    /// inode 最后修改时间
    ///
    /// Unix 时间戳
    _ctime: u32,
    /// 数据最后修改时间
    ///
    /// Unix 时间戳
    _mtime: u32,
    /// 文件删除时间
    ///
    /// Unix 时间戳
    _dtime: u32,
    /// 所有者的组 id
    _gid: u16,
    /// 硬链接数量
    _links_count: u16,
    /// 逻辑块数量，一个逻辑块为 512 字节，这个字段表示文件实际上占用了多少 512 字节
    pub logical_block_count: u32,
    /// 文件标志
    _flags: u32,
    /// 操作系统特定字段
    _os_specific: u32,
    /// 直接块指针，指向实际数据块
    pub direct_blocks: [u32; 12],
    /// 单间接块指针
    pub level1_indirect_block: u32,
    /// 双间接块指针
    pub level2_indirect_block: u32,
    /// 三间接块指针
    pub level3_indirect_block: u32,
    /// 文件版本号
    _generation: u32,
    /// 文件 acl 所在块号
    _file_acl: u32,
    /// 目录 acl 所在块号
    _dir_acl: u32,
    /// 文件碎片地址
    _faddr: u32,
    /// 操作系统相关字段
    _osd2: [u8; 12],
}
