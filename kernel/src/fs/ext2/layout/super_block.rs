use zerocopy::{FromBytes, IntoBytes, KnownLayout};

#[derive(Debug, Clone, IntoBytes, FromBytes, KnownLayout)]
#[repr(C)]
pub struct SuperBlock {
    /// 整个文件系统的 inode 总数
    pub inodes_count: u32,
    /// 整个文件系统块的总数
    pub blocks_count: u32,
    /// 保留块的总数
    _reserved_blocks_count: u32,
    /// 空闲块的数量
    pub free_blocks_count: u32,
    /// 空闲 inode 的数量
    pub free_inodes_count: u32,
    /// 包含 super block 的块号
    _first_data_block: u32,
    /// 块大小的对数
    ///
    /// 记这个数为 x，则块大小为 1024 << x 字节（等价于 1024 * 2^x）
    pub log_block_size: u32,
    /// 碎片大小的对数
    ///
    /// 与 `log_block_size` 相同方式计算 fragment 大小
    _log_fragment_size: u32,
    /// 每个块组的块数量
    pub blocks_per_group: u32,
    /// 每个块组内的碎片数量
    _fragments_per_group: u32,
    /// 每个块组的 inode 数量
    pub inodes_per_group: u32,
    /// 文件系统最后一次挂载时间
    ///
    /// Unix 时间戳
    _mtime: u32,
    /// 文件系统最后一次写入时间
    _wtime: u32,
    /// 自上次文件系统一致性检查以来，文件系统被挂载的次数
    _mnt_count: u16,
    /// 强制进行文件系统检查的最大挂载次数
    _max_mnt_count: u16,
    /// 魔数
    pub magic: u16,
    /// 文件系统的状态
    _state: u16,
    /// 文件系统检测到错误时，内核应采取的处理行为
    _errors: u16,
    /// 版本号的 minor 部分
    _minor_rev_level: u16,
    /// 上次一致性检查的时间
    _lastcheck: u32,
    /// 一致性检查的间隔时间
    _checkinterval: u32,
    /// 创建文件系统的操作系统
    _creator_os: u32,
    /// 版本号的 major 部分
    _major_rev_level: u32,
    /// 可以使用保留块的 uid
    _reserved_block_uid: u16,
    /// 可以使用保留块的 gid
    _reserved_block_gid: u16,
    /// 第一个非保留 inode 的编号
    _first_inode: u32,
    /// 磁盘上 inode 结构的大小
    pub inode_size: u16,
    /// 本超级块副本所在的块组号
    _block_group_nr: u16,
    /// 兼容特性位图
    _feature_compat: u32,
    /// 不兼容特性位图
    _feature_incompat: u32,
    /// 只读兼容特性位图
    _feature_ro_compat: u32,
    /// 卷 UUID
    _uuid: [u8; 16],
    /// 卷名（以 null 结尾的 ASCII）
    pub volume_name: [u8; 16],
    /// 上次挂载路径（以 null 结尾的 ASCII）
    _last_mounted: [u8; 64],
    /// 压缩等算法使用位图
    _algorithm_usage_bitmap: u32,
    /// 为常规文件预分配的块数（性能提示）
    _prealloc_blocks: u8,
    /// 为目录预分配的块数（性能提示）
    _prealloc_dir_blocks: u8,
    /// 对齐填充
    _padding1: u16,
    /// 日志超级块的 UUID（若启用日志）
    _journal_uuid: [u8; 16],
    /// 日志文件的 inode 号
    _journal_inode: u32,
    /// 日志所在设备号
    _journal_dev: u32,
    /// 待删除 inode 链表头
    _last_orphan: u32,
    _reserved: [u8; 788],
}
