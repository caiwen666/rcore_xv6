use core::fmt::Debug;
use core::ops::{Add, Sub};

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[repr(transparent)]
pub struct PhysAddr(usize);

impl PhysAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// 获取对应的可变引用
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }

    /// 获取对应的不可变引用
    pub fn get<T>(&self) -> &'static T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }

    /// 检查是否满足对齐要求
    ///
    /// # Parameters
    ///
    /// - `align`: 对齐要求，必须是 2 的幂次
    #[expect(unused)]
    pub fn check_aligned(&self, align: usize) -> bool {
        self.0 & (align - 1) == 0
    }

    pub fn inner(&self) -> usize {
        self.0
    }
}

impl Add<usize> for PhysAddr {
    type Output = Self;
    fn add(self, other: usize) -> Self::Output {
        Self(self.0 + other)
    }
}

impl Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PhysAddr(0x{:X})", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[repr(transparent)]
pub struct VirtAddr(usize);

impl VirtAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// 从引用创建虚拟地址
    pub fn from_ref<T>(r: &T) -> Self {
        Self(r as *const T as usize)
    }

    /// 检查是否满足对齐要求
    ///
    /// # Parameters
    ///
    /// - `align`: 对齐要求，必须是 2 的幂次
    #[expect(unused)]
    pub fn check_aligned(&self, align: usize) -> bool {
        self.0 & (align - 1) == 0
    }

    pub fn inner(&self) -> usize {
        self.0
    }
}

impl Add<usize> for VirtAddr {
    type Output = Self;
    fn add(self, other: usize) -> Self::Output {
        Self(self.0 + other)
    }
}

impl Sub<VirtAddr> for VirtAddr {
    type Output = usize;
    fn sub(self, other: Self) -> Self::Output {
        self.0 - other.0
    }
}

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VirtAddr(0x{:X})", self.0)
    }
}
