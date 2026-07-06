use alloc::sync::Arc;

use crate::{
    console::{Stdin, Stdout},
    error::SystemError,
    fs::vfs::{VirtualFile, VirtualIndexNode, lookup},
    process::ProcessControlBlock,
};

pub enum FileSeekMethod {
    Absolute(usize),
    #[expect(unused)]
    Relative(isize),
    End(isize),
}

pub trait File: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> Result<usize, SystemError>;
    fn write(&self, buf: &[u8]) -> Result<usize, SystemError>;
    fn seek(&self, method: FileSeekMethod) -> Result<usize, SystemError>;
}

impl ProcessControlBlock {
    /// 在进程中打开一个文件，返回文件描述符
    ///
    /// **该函数会对 inner 加锁**
    pub fn open_file(&self, path: &str) -> Option<u64> {
        let file: Arc<dyn File> = if path == "stdin" {
            Arc::new(Stdin)
        } else if path == "stdout" {
            Arc::new(Stdout)
        } else {
            let cwd = self.cwd();
            let inode = lookup(cwd, path)?;
            Arc::new(VirtualFile::new(inode))
        };
        let mut inner = self.inner();
        let fd = inner.fd_table.push(file);
        Some(fd as u64)
    }

    /// **该函数会对 inner 加锁**
    pub fn get_file(&self, fd: u64) -> Option<Arc<dyn File>> {
        let inner = self.inner();
        inner.fd_table.get(fd as usize).map(|f| f.clone())
    }
}
