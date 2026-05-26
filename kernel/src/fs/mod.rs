pub mod ramfs;
pub mod vfs;

use crate::fs::{ramfs::RamFS, vfs::mount::MountFS};
use alloc::{string::ToString, sync::Arc};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ROOT_FS: Arc<MountFS> = {
        let ramfs = RamFS::new();
        let root = ramfs.root();
        root.push_file(
            "logo.txt".to_string(),
            include_bytes!("../logo.txt").to_vec(),
        );
        root.push_directory("root".to_string());
        let test = root.push_directory("test".to_string());
        test.push_file("test.txt".to_string(), "test".as_bytes().to_vec());
        MountFS::new_root(Arc::new(ramfs))
    };
}
