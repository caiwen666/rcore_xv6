use alloc::sync::Arc;

use crate::{
    console::{Stdin, Stdout},
    error::SystemError,
    fs::vfs::{VirtualFile, interface::DirectoryEntry, lookup},
    process::ProcessControlBlock,
};

#[expect(unused)]
pub enum FileSeekMethod {
    Absolute(usize),
    Relative(isize),
    End(isize),
}

pub enum ReadDirControl {
    /// 继续遍历
    Continue,
    /// 停止遍历
    #[expect(unused)]
    Stop,
    /// 停止遍历，同时让 offset 不因此次遍历而向前推进
    StopWithBackroll,
}

pub trait File: Send + Sync {
    fn read(&self, _buf: &mut [u8]) -> Result<usize, SystemError> {
        Err(SystemError::EBADF)
    }
    fn write(&self, _buf: &[u8]) -> Result<usize, SystemError> {
        Err(SystemError::EBADF)
    }
    #[expect(unused)]
    fn seek(&self, _method: FileSeekMethod) -> Result<usize, SystemError> {
        Err(SystemError::EBADF)
    }
    fn read_dir(
        &self,
        _f: &mut dyn FnMut(DirectoryEntry, &mut ReadDirControl) -> Result<(), SystemError>,
    ) -> Result<(), SystemError> {
        Err(SystemError::ENOTDIR)
    }
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
    #[expect(unused)]
    pub fn get_file(&self, fd: u64) -> Option<Arc<dyn File>> {
        let inner = self.inner();
        inner.fd_table.get(fd as usize).cloned()
    }
}
