use super::{Inode, layout};
use crate::fs::vfs::interface;
use alloc::sync::{Arc, Weak};
use zerocopy::FromBytes;

const EXT2_SUPER_MAGIC: u16 = 0xEF53;

pub struct FileSystem {
    block_device: Arc<dyn interface::BlockDevice>,
    super_block: layout::SuperBlock,
    block_size: u32,
    weak_self: Weak<Self>,
}

fn read_super_block(block_device: &Arc<dyn interface::BlockDevice>) -> layout::SuperBlock {
    // 第一个 1024 字节是 boot block，第二个 1024 字节是 superblock
    let mut super_block_buf = [0u8; 1024];
    block_device.read_at(1024, &mut super_block_buf);
    layout::SuperBlock::read_from_bytes(&super_block_buf).expect("Failed to parse superblock")
}

impl FileSystem {
    /// # Panics
    ///
    /// - 如果魔数不正确，则 panic
    /// - 如果块组数量不正确，则 panic
    ///
    /// # Preconditions
    ///
    /// 调用者需要确保一个块设备只能挂载一个文件系统示例，否则会出现数据竞争问题
    pub fn new(block_device: Arc<dyn interface::BlockDevice>) -> Arc<Self> {
        let super_block = read_super_block(&block_device);
        assert_eq!(
            super_block.magic, EXT2_SUPER_MAGIC,
            "EXT2: Unexpected superblock magic number: 0x{:X}",
            super_block.magic
        );

        let block_size = 1024 << super_block.log_block_size;
        // 最后一个块组的块数量可能没满，所以需要上取整
        let block_group_count = super_block
            .blocks_count
            .div_ceil(super_block.blocks_per_group);
        let block_group_count_2 = super_block
            .inodes_count
            .div_ceil(super_block.inodes_per_group);
        assert_eq!(
            block_group_count, block_group_count_2,
            "EXT2: Unexpected block group count: {} != {}",
            block_group_count, block_group_count_2
        );

        assert_eq!(
            super_block.inode_size, 128,
            "EXT2: Unexpected inode size: {}",
            super_block.inode_size
        );

        Arc::new_cyclic(|weak_self| Self {
            block_device,
            super_block,
            block_size,
            weak_self: weak_self.clone(),
        })
    }

    pub fn block_device(&self) -> &Arc<dyn interface::BlockDevice> {
        &self.block_device
    }

    pub fn block_size(&self) -> usize {
        self.block_size as usize
    }

    fn read_group_desc(
        &self,
        block_device: &Arc<dyn interface::BlockDevice>,
        group_id: u32,
    ) -> layout::GroupDescriptor {
        let gdt_block = 2048u32.div_ceil(self.block_size);
        let addr = gdt_block * self.block_size + group_id * 32;
        let mut group_desc_buf = [0u8; 32];
        block_device.read_at(addr as usize, &mut group_desc_buf);
        layout::GroupDescriptor::read_from_bytes(&group_desc_buf)
            .expect("Failed to parse group descriptor")
    }
}

impl interface::FileSystem for FileSystem {
    fn root_inode(&self) -> u64 {
        2
    }

    fn get_inode(&self, inode_id: u64) -> Arc<dyn interface::IndexNode> {
        let inode_id = inode_id as u32;
        let group_id = (inode_id - 1) / self.super_block.inodes_per_group;
        let local_index = (inode_id - 1) % self.super_block.inodes_per_group;
        let group_desc = self.read_group_desc(&self.block_device, group_id);
        let inode_byte_offset = local_index * self.super_block.inode_size as u32;
        let inode_addr = group_desc.inode_table * self.block_size + inode_byte_offset;
        let mut inode_buf = [0u8; 128];
        self.block_device
            .read_at(inode_addr as usize, &mut inode_buf);
        let inode_info: layout::Inode =
            layout::Inode::read_from_bytes(&inode_buf).expect("Failed to parse inode");
        Arc::new(Inode::new(
            self.weak_self.upgrade().unwrap().clone(),
            inode_info,
            inode_id,
        ))
    }

    fn name(&self) -> &'static str {
        "ext2"
    }
}
