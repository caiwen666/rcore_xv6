pub mod ramfs;
pub mod vfs;

use crate::fs::{ramfs::RamFS, vfs::VirtualFileSystem};
use alloc::{string::ToString, sync::Arc};
use lazy_static::lazy_static;

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
