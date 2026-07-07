#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemError {
    /// 操作不被允许
    /// Operation not permitted.
    EPERM = 1,
    /// 没有指定的文件或目录。
    /// No such file or directory.
    ENOENT = 2,
    /// 被中断的函数
    /// Interrupted function.
    EINTR = 4,
    /// 错误的文件描述符
    /// Bad file descriptor.
    EBADF = 9,
    /// 没有足够的空间
    /// Not enough space.
    ENOMEM = 12,
    /// 用户态地址不可访问
    /// Bad address.
    EFAULT = 14,
    /// 不是一个目录
    /// Not a directory.
    ENOTDIR = 20,
    /// 非法参数
    /// Invalid argument.
    EINVAL = 22,
    /// 结果过大
    /// Result too large.
    ERANGE = 34,
    /// 路径名太长
    /// Filename too long.
    ENAMETOOLONG = 36,
}

impl SystemError {
    pub fn posix_errno(&self) -> isize {
        -(*self as isize)
    }
}
