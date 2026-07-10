use crate::{
    fs::{
        ext2::layout,
        vfs::{
            FileSystem,
            interface::{self, DirectoryEntry},
        },
    },
    utils::BlockIterator,
};
use alloc::vec;
use alloc::{string::ToString, sync::Arc};
use zerocopy::FromBytes;

use super::fs::FileSystem as Ext2FileSystem;

pub struct Inode {
    fs: Arc<Ext2FileSystem>,
    layout: layout::Inode,
    id: u32,
}

impl Inode {
    pub fn new(fs: Arc<Ext2FileSystem>, layout: layout::Inode, id: u32) -> Self {
        Self { fs, layout, id }
    }

    fn map_logical_block(&self, logical_block_id: u32) -> u32 {
        // 每个间接块内的指针个数
        let ptrs = self.fs.block_size() as u32 / 4;
        if logical_block_id < 12 {
            return self.layout.direct_blocks[logical_block_id as usize];
        }
        let mut lb = logical_block_id - 12;
        // 一级间接
        if lb < ptrs {
            if self.layout.level1_indirect_block == 0 {
                return 0;
            }
            let mut block_id_buf = [0u8; 4];
            self.fs.block_device().read_at(
                self.layout.level1_indirect_block as usize * self.fs.block_size() + lb as usize * 4,
                &mut block_id_buf,
            );
            return u32::from_le_bytes(block_id_buf);
        }
        lb -= ptrs;
        // 二级间接
        if lb < ptrs * ptrs {
            if self.layout.level2_indirect_block == 0 {
                return 0;
            }
            let mut level1_indirect_block_id_buf = [0u8; 4];
            let level2_idx = lb / ptrs;
            self.fs.block_device().read_at(
                self.layout.level2_indirect_block as usize * self.fs.block_size()
                    + level2_idx as usize * 4,
                &mut level1_indirect_block_id_buf,
            );
            let level1_indirect_block_id = u32::from_le_bytes(level1_indirect_block_id_buf);
            if level1_indirect_block_id == 0 {
                return 0;
            }
            let mut block_id_buf = [0u8; 4];
            let level1_idx = lb % ptrs;
            self.fs.block_device().read_at(
                level1_indirect_block_id as usize * self.fs.block_size() + level1_idx as usize * 4,
                &mut block_id_buf,
            );
            return u32::from_le_bytes(block_id_buf);
        }
        lb -= ptrs * ptrs;
        // 三级间接
        assert!(lb < ptrs * ptrs * ptrs);
        if self.layout.level3_indirect_block == 0 {
            return 0;
        }
        let mut level2_indirect_block_id_buf = [0u8; 4];
        let level3_idx = lb / (ptrs * ptrs);
        self.fs.block_device().read_at(
            self.layout.level3_indirect_block as usize * self.fs.block_size()
                + level3_idx as usize * 4,
            &mut level2_indirect_block_id_buf,
        );
        let level2_indirect_block_id = u32::from_le_bytes(level2_indirect_block_id_buf);
        if level2_indirect_block_id == 0 {
            return 0;
        }
        let mut level1_indirect_block_id_buf = [0u8; 4];
        let level2_idx = lb % (ptrs * ptrs) / ptrs;
        self.fs.block_device().read_at(
            level2_indirect_block_id as usize * self.fs.block_size() + level2_idx as usize * 4,
            &mut level1_indirect_block_id_buf,
        );
        let level1_indirect_block_id = u32::from_le_bytes(level1_indirect_block_id_buf);
        if level1_indirect_block_id == 0 {
            return 0;
        }
        let mut block_id_buf = [0u8; 4];
        let level1_idx = lb % (ptrs * ptrs) % ptrs;
        self.fs.block_device().read_at(
            level1_indirect_block_id as usize * self.fs.block_size() + level1_idx as usize * 4,
            &mut block_id_buf,
        );
        u32::from_le_bytes(block_id_buf)
    }
}

/// 从 `buf` 的 `offset` 处读取一个目录项
///
/// # Returns
///
/// - 返回下一个目录项的偏移量、目录项的头部和目录项的名称
/// - 如果读取失败（如偏移量超出范围），则返回 None
fn read_dir_entry(buf: &[u8], mut offset: usize) -> Option<(usize, layout::DirEntryHeader, &str)> {
    loop {
        if offset + 8 > buf.len() {
            return None;
        }
        let header = layout::DirEntryHeader::read_from_bytes(&buf[offset..offset + 8])
            .expect("EXT2: failed to parse directory entry header");
        // 跳过被删除的目录项
        if header.inode == 0 {
            offset += header.rec_len as usize;
            continue;
        }
        let name_end = offset + 8 + header.name_len as usize;
        if name_end > buf.len() {
            return None;
        }
        let entry_name = core::str::from_utf8(&buf[offset + 8..name_end]).ok()?;
        return Some((offset + header.rec_len as usize, header, entry_name));
    }
}

impl interface::IndexNode for Inode {
    fn resize(&self, _new_size: usize) {
        todo!()
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) {
        assert!(offset + buf.len() <= self.layout.size as usize);
        let block_size = self.fs.block_size();
        let mut pos = 0;
        for block in BlockIterator::new(block_size, offset, buf.len()) {
            let block_id = self.map_logical_block(block.block_id() as u32);
            if block_id == 0 {
                buf[pos..pos + block.size()].fill(0);
            } else {
                self.fs.block_device().read_at(
                    block_id as usize * block_size + block.offset(),
                    &mut buf[pos..pos + block.size()],
                );
            }
            pos += block.size();
        }
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) {
        todo!()
    }

    fn find(&self, name: &str) -> Option<u64> {
        let dir_size = self.layout.size as usize;
        let mut data = vec![0u8; dir_size];
        self.read_at(0, &mut data);
        let mut offset = 0;
        loop {
            let (next_offset, header, entry_name) = read_dir_entry(&data, offset)?;
            if entry_name == name {
                return Some(header.inode as u64);
            }
            offset = next_offset;
        }
    }

    fn parent(&self) -> Option<u64> {
        let inode_id = self.find("..").unwrap();
        if inode_id == self.id as u64 {
            None
        } else {
            Some(inode_id)
        }
    }

    fn metadata(&self) -> interface::Metadata {
        const FILE_TYPE_MASK: u16 = 0xF000;
        const TYPE_FILE: u16 = 0x8000;
        const TYPE_DIR: u16 = 0x4000;
        interface::Metadata {
            file_type: match self.layout.mode & FILE_TYPE_MASK {
                TYPE_FILE => interface::FileType::File,
                TYPE_DIR => interface::FileType::Directory,
                _ => panic!(
                    "EXT2: Invalid file type: 0x{:X}",
                    self.layout.mode & FILE_TYPE_MASK
                ),
            },
            size: self.layout.size as usize,
        }
    }

    fn read_dir(&self, offset_cookie: u64) -> Option<DirectoryEntry> {
        let dir_size = self.layout.size as usize;
        let mut data = vec![0u8; dir_size];
        // TODO 这里实际上还是把整个目录都读了，需要考虑优化
        self.read_at(0, &mut data);
        let (next_offset, header, name) = read_dir_entry(&data, offset_cookie as usize)?;
        let inode = self.fs.get_inode(header.inode as u64);
        Some(DirectoryEntry {
            name: name.to_string(),
            offset_cookie: next_offset as u64,
            inode: header.inode as u64,
            file_type: inode.metadata().file_type,
        })
    }

    fn id(&self) -> u64 {
        self.id as u64
    }
}
