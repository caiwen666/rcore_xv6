use alloc::sync::Arc;

use crate::{
    console::{Stdin, Stdout},
    fs::vfs::{VirtualFile, lookup},
    process::ProcessResource,
    sync::spin::SpinMutex,
};

pub enum FileSeekMethod {
    Absolute(usize),
    #[expect(unused)]
    Relative(isize),
    End(isize),
}

pub trait File: Send + Sync {
    /// 如果不支持读取则返回 None
    fn read(&self, buf: &mut [u8]) -> Option<usize>;
    /// 如果不支持写入则返回 None
    fn write(&self, buf: &[u8]) -> Option<usize>;
    /// 移动文件指针，返回新的文件指针位置，如果不能移动则返回 None
    fn seek(&self, method: FileSeekMethod) -> Option<usize>;
}

impl SpinMutex<ProcessResource> {
    /// 在进程中打开一个文件，返回文件描述符
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
        let mut resource = self.lock();
        let fd = resource.avail_fd.alloc();
        if fd >= resource.fd_table.len() {
            resource.fd_table.push(Some(file));
        } else {
            resource.fd_table[fd] = Some(file);
        }
        Some(fd as u64)
    }

    pub fn get_file(&self, fd: u64) -> Option<Arc<dyn File>> {
        let resource = self.lock();
        resource
            .fd_table
            .get(fd as usize)
            .and_then(|f| f.as_ref().cloned())
    }
}
