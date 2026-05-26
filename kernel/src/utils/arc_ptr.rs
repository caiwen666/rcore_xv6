use alloc::sync::Arc;
use core::cmp::Ordering;

/// 将 Arc 包装为可比较的类型，以用于 BTreeMap 的 key 之类的情况。
#[derive(Clone)]
pub struct ArcPtr<T: ?Sized> {
    inner: Arc<T>,
}

impl<T: ?Sized> ArcPtr<T> {
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

impl<T: ?Sized> PartialEq for ArcPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}
impl<T: ?Sized> Eq for ArcPtr<T> {}

impl<T: ?Sized> PartialOrd for ArcPtr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: ?Sized> Ord for ArcPtr<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        Arc::as_ptr(&self.inner)
            // 由于 T 为 ?Sized，所以 Arc<T> 在内存上的布局实际上是一个胖指针，
            // 胖指针会包含一个指向堆上数据的指针，以及附加的元数据。
            // 这里 Arc::as_ptr 返回 *const T 实际上是一个胖指针
            // 直接比较相等会要求堆上地址和胖指针附加数据均相同
            // 这里会报 warn，Rust 担心直接比较胖指针可能不是我们想要的
            // 比如如果 T 是切片的话，那么切片长度不同的两个 Arc<T> 对应的 as_ptr
            // 实际上是不相等的。
            // 这里使用 cast 直接将胖指针转为细指针，Rust 约定，转为的细指针会
            // 剔除掉附加的元数据
            .cast::<()>()
            .cmp(&Arc::as_ptr(&other.inner).cast::<()>())
    }
}
