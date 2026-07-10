use crate::{
    error::SystemError,
    fs::{
        file::{File, FileSeekMethod},
        vfs::{VirtualIndexNode, interface::FileType},
    },
    sync::mutex::Mutex,
};

pub struct VirtualFile {
    inner: Mutex<VirtualFileInner>,
}

struct VirtualFileInner {
    offset: usize,
    inode: VirtualIndexNode,
}

impl VirtualFile {
    pub fn new(inode: VirtualIndexNode) -> Self {
        Self {
            inner: Mutex::new(VirtualFileInner { offset: 0, inode }, "virtual_file_inner"),
        }
    }
}

impl File for VirtualFile {
    fn read(&self, buf: &mut [u8]) -> Result<usize, SystemError> {
        let mut inner = self.inner.lock();
        if inner.inode.metadata().file_type != FileType::File {
            return Err(SystemError::EISDIR);
        }
        let len = inner.inode.read_at(inner.offset, buf);
        inner.offset += len;
        Ok(len)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, SystemError> {
        let mut inner = self.inner.lock();
        if inner.inode.metadata().file_type != FileType::File {
            return Err(SystemError::EISDIR);
        }
        let file_size = inner.inode.metadata().size;
        let now_offset = inner.offset;
        if now_offset >= file_size {
            inner.inode.resize(now_offset + 1);
        }
        let len = inner.inode.write_at(now_offset, buf);
        inner.offset += len;
        Ok(len)
    }

    fn seek(&self, method: FileSeekMethod) -> Result<usize, SystemError> {
        let mut inner = self.inner.lock();
        let pos = match method {
            FileSeekMethod::Absolute(pos) => pos,
            FileSeekMethod::Relative(offset) => {
                if offset < 0 && inner.offset < offset.unsigned_abs() {
                    0
                } else {
                    (inner.offset as isize + offset) as usize
                }
            }
            FileSeekMethod::End(offset) => (inner.inode.metadata().size as isize + offset) as usize,
        };
        inner.offset = pos;
        Ok(pos)
    }
}
