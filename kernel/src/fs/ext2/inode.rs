use super::fs::FileSystem;
use crate::{
    fs::{ext2::layout, vfs::interface},
    utils::BlockIterator,
};
use alloc::vec;
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use zerocopy::FromBytes;

pub struct Inode {
    fs: Arc<FileSystem>,
    layout: layout::Inode,
    id: u32,
}

impl Inode {
    pub fn new(fs: Arc<FileSystem>, layout: layout::Inode, id: u32) -> Self {
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
            let mut level1_indirect_block_id_buf = [0u8; 4];
            let level1_idx = lb / ptrs;
            self.fs.block_device().read_at(
                self.layout.level2_indirect_block as usize * self.fs.block_size()
                    + level1_idx as usize * 4,
                &mut level1_indirect_block_id_buf,
            );
            let level1_indirect_block_id = u32::from_le_bytes(level1_indirect_block_id_buf);
            let level2_idx = lb % ptrs;
            let mut block_id_buf = [0u8; 4];
            self.fs.block_device().read_at(
                level1_indirect_block_id as usize * self.fs.block_size() + level2_idx as usize * 4,
                &mut block_id_buf,
            );
            return u32::from_le_bytes(block_id_buf);
        }
        lb -= ptrs * ptrs;
        // 三级间接
        assert!(lb < ptrs * ptrs * ptrs);
        let mut level2_indirect_block_id_buf = [0u8; 4];
        let level1_idx = lb / (ptrs * ptrs);
        self.fs.block_device().read_at(
            self.layout.level3_indirect_block as usize * self.fs.block_size()
                + level1_idx as usize * 4,
            &mut level2_indirect_block_id_buf,
        );
        let level2_indirect_block_id = u32::from_le_bytes(level2_indirect_block_id_buf);
        let mut level1_indirect_block_id_buf = [0u8; 4];
        let level2_idx = lb % (ptrs * ptrs) / ptrs;
        self.fs.block_device().read_at(
            level2_indirect_block_id as usize * self.fs.block_size() + level2_idx as usize * 4,
            &mut level1_indirect_block_id_buf,
        );
        let level1_indirect_block_id = u32::from_le_bytes(level1_indirect_block_id_buf);
        let mut block_id_buf = [0u8; 4];
        let level3_idx = lb % (ptrs * ptrs) % ptrs;
        self.fs.block_device().read_at(
            level1_indirect_block_id as usize * self.fs.block_size() + level3_idx as usize * 4,
            &mut block_id_buf,
        );
        u32::from_le_bytes(block_id_buf)
    }
}

/// 遍历目录项
///
/// # Parameters
///
/// - `buf`: 目录项列表的数据
/// - `f`: 每次遍历时的回调函数。每遍历到一个目录项就会调用该函数，传递该目录项的头部和名称
///   该函数应返回一个 bool 类型，表示还要不要继续遍历。
fn read_dir_entries(buf: &[u8], mut f: impl FnMut(&layout::DirEntryHeader, &str) -> bool) {
    let mut offset = 0;
    while offset + 8 <= buf.len() {
        let header = layout::DirEntryHeader::read_from_bytes(&buf[offset..offset + 8])
            .expect("EXT2: failed to parse directory entry header");
        let name_end = offset + 8 + header.name_len as usize;
        if header.inode != 0 {
            let entry_name = core::str::from_utf8(&buf[offset + 8..name_end])
                .expect("EXT2: invalid directory entry name");
            if !f(&header, entry_name) {
                break;
            }
        }
        offset += header.rec_len as usize;
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
            self.fs.block_device().read_at(
                block_id as usize * block_size + block.offset(),
                &mut buf[pos..pos + block.size()],
            );
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
        let mut inode_id = None;
        read_dir_entries(&data, |header, entry_name| {
            if name == entry_name {
                inode_id = Some(header.inode as u64);
                return false;
            }
            true
        });
        inode_id
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

    fn list(&self) -> Vec<String> {
        let mut list = Vec::new();
        let dir_size = self.layout.size as usize;
        let mut data = vec![0u8; dir_size];
        self.read_at(0, &mut data);
        read_dir_entries(&data, |_header, name| {
            list.push(name.to_string());
            true
        });
        list
    }

    fn id(&self) -> u64 {
        self.id as u64
    }
}
