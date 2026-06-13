mod ext2;
pub mod file;
pub mod ramfs;
pub mod vfs;

use crate::{
    driver::virtio::{device::blk::VirtIOBlk, transport::Transport},
    fs::{ramfs::RamFS, vfs::VirtualFileSystem},
};
use alloc::{string::ToString, sync::Arc};
use lazy_static::lazy_static;

impl<T: Transport + Send + 'static> vfs::interface::BlockDevice for VirtIOBlk<T> {
    fn block_size(&self) -> usize {
        512
    }

    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        VirtIOBlk::<T>::read_block(self, block_id, buf);
    }

    fn write_block(&self, _block_id: usize, _buf: &[u8]) {
        todo!()
    }
}

lazy_static! {
    pub static ref ROOT_FS: Arc<VirtualFileSystem> = {
        let (ramfs, root) = RamFS::new();
        root.push_file(
            "logo.txt".to_string(),
            include_bytes!("../logo.txt").to_vec(),
        );
        root.push_directory("root".to_string());
        let test = root.push_directory("test".to_string());
        test.push_file("test.txt".to_string(), "test".as_bytes().to_vec());
        VirtualFileSystem::new(ramfs)
    };
}

pub use ext2::FileSystem as Ext2FileSystem;
