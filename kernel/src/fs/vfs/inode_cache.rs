use core::{ptr::NonNull, time::Duration};

use alloc::boxed::Box;

use crate::{
    fs::vfs::{VirtualIndexNodeInner, VirtualIndexNodeInnerLocked, page_cache::PageCache},
    process::{
        kthread::{exit_kthread, spawn_kthread},
        timer::sleep_with_interval,
    },
    sync::{condvar::Condvar, spin::SpinMutex},
};

use super::{VirtualFileSystem, VirtualIndexNode};

impl VirtualFileSystem {
    fn new_inode(&self, id: u64) -> VirtualIndexNode {
        let inner = Box::new(VirtualIndexNodeInner {
            condvar: Condvar::new(),
            inner_locked: SpinMutex::new(
                VirtualIndexNodeInnerLocked {
                    inode: None,
                    ref_count: 1,
                    destroying: false,
                },
                "inode_inner_locked",
            ),
            page_cache: PageCache::new(),
            id,
        });
        VirtualIndexNode {
            ptr: unsafe { NonNull::new_unchecked(Box::leak(inner)) },
            fs: self.self_weak.clone(),
            weak: false,
        }
    }
    /// 获取一个 inode 实例，如果 inode 已经被缓存则直接返回，
    /// 否则的话会调用具体文件系统的 inode 并缓存
    ///
    /// **会阻塞**
    pub(super) fn get_inode_with_cache(&self, id: u64) -> VirtualIndexNode {
        let mut cache = self.inode_cache.lock();
        if let Some(inode) = cache.get(&id).cloned() {
            drop(cache);
            let mut inner_locked = inode.inner_locked.lock();
            // 如果 inode 正在初始化中，那么需要等待
            if inner_locked.inode.is_none() {
                inner_locked = inode.condvar.wait(inner_locked);
            }
            // 不需要 while，只要被唤醒了就说明 inode 已经初始化好了
            assert!(inner_locked.inode.is_some());
            drop(inner_locked);
            return inode;
        }
        // 没有缓存的话，就从缓存表中创建一个占位的
        let inode = self.new_inode(id);
        cache.insert(id, inode.clone());
        // 调用 get_inode 时，不能持有自旋锁
        drop(cache);
        // 这里会存在堵塞
        let inner_inode = self.inner_fs.get_inode(id);
        let mut inner_locked = inode.inner_locked.lock();
        inner_locked.inode = Some(inner_inode);
        inode.condvar.notify_all();
        drop(inner_locked);
        inode
    }
}

impl VirtualIndexNode {
    /// 和普通 clone 大致相同，但是不会清除 destroying 标记
    ///
    /// 克隆出来的 [VirtualIndexNode] 不能再被 clone，同时在 drop 的时候不检查引用计数是否减为了 1
    ///
    /// # Panics
    ///
    /// 如果当前 [VirtualIndexNode] 是 weak 的，则 panic
    fn weak_clone(&self) -> VirtualIndexNode {
        assert!(!self.weak);
        let inode = unsafe { self.ptr.as_ref() };
        let mut inner_locked = inode.inner_locked.lock();
        inner_locked.ref_count += 1;
        VirtualIndexNode {
            ptr: NonNull::from_ref(inode),
            fs: self.fs.clone(),
            weak: true,
        }
    }
}
impl Clone for VirtualIndexNode {
    fn clone(&self) -> Self {
        assert!(!self.weak);
        let inode = unsafe { self.ptr.as_ref() };
        let mut inner_locked = inode.inner_locked.lock();
        // 有可能是从 inode_cache 中克隆，此时必然会正在持有 inode_cache 的锁，这里就不要再拿锁
        inner_locked.ref_count += 1;
        // inner_locked.destroying 如果为 true 的话，则必然是引用计数从 1 -> 2 这个 clone 过程
        // 而这个 clone 过程必然是从 inode_cache 中克隆的，此时必然是持有 inode_cache 的锁
        // 这里应该不存在和 inode 回收竞争的情况
        if inner_locked.destroying {
            inner_locked.destroying = false;
        }
        VirtualIndexNode {
            ptr: NonNull::from_ref(inode),
            fs: self.fs.clone(),
            weak: false,
        }
    }
}
impl Drop for VirtualIndexNode {
    fn drop(&mut self) {
        let inode = unsafe { self.ptr.as_ref() };
        let mut inner_locked = inode.inner_locked.lock();
        inner_locked.ref_count -= 1;
        // 此时只剩下了在 inode_cache 中的引用了，我们认为此时可以开始准备释放 inode 了
        if !self.weak && inner_locked.ref_count == 1 {
            inner_locked.destroying = true;
            // 由于 weak clone 还要用到 inner_locked，所以这里先释放锁
            drop(inner_locked);
            let weak_inode = self.weak_clone();
            spawn_kthread(move || {
                {
                    // 这里的 sync 应该是会直接把整个 inode 的全部 dirty page 全部落盘的
                    // 因为现在引用计数为 1，之后 inode_cache 里面存在引用，所以不会存在
                    // 中间又产生 dirty_page 的情况
                    // 如果真的出现了产生 dirty_page，那么就说明 inode 肯定被从 inode_cache
                    // 中克隆出去了，那么 destroying 标记会被清除，销毁不会继续进行。
                    weak_inode.sync();
                    // TODO 这里模拟耗时操作
                    sleep_with_interval(Duration::from_secs(5));
                    // 先对 inode_cache 加锁，再判断 destroying 标记是否存在
                    let fs = weak_inode.fs();
                    let mut inode_cache = fs.inode_cache.lock();
                    let inner_locked = weak_inode.inner_locked.lock();
                    // 如果标记被清除的话，说明在销毁过程中又有人从 inode_cache 中克隆出来 inode 了
                    // 就不再继续回收
                    if inner_locked.destroying {
                        let cache_inode = inode_cache.remove(&weak_inode.id).unwrap();
                        // 先释放 weak_inode 再释放 cache_inode，cache_inode 不是 weak 的，如果先释放他的话
                        // 引用计数又归 1，那么 drop 中又会跑到这里，就死循环了
                        drop(inner_locked);
                        drop(weak_inode);
                        drop(cache_inode);
                    }
                }
                exit_kthread();
            });
        } else if inner_locked.ref_count == 0 {
            // 这里要先释放 inner_locked，不这样写出来的话，后面 _boxed 先把堆上数据释放
            // 然后 inner_locked 就成野指针了
            drop(inner_locked);
            // 此时说明在 inode cache 里面的 Arc 也释放了，整个 inode 都可以释放了
            let _boxed = unsafe { Box::from_raw(self.ptr.as_ptr() as *mut _) };
        }
    }
}

impl VirtualIndexNode {
    /// 在当前文件系统中寻找 inode，不会根据挂载点进行跳转
    ///
    /// **会堵塞**
    pub(super) fn find_with_cache(&self, name: &str) -> Option<VirtualIndexNode> {
        let inner_locked = self.inner_locked.lock();
        let id = inner_locked.inode().find(name)?;
        drop(inner_locked);
        let fs = self.fs();
        Some(fs.get_inode_with_cache(id))
    }
}
