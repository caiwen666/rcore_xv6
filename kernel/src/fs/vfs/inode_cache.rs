use core::{ptr::NonNull, time::Duration};

use alloc::boxed::Box;

use crate::{
    fs::vfs::{VirtualIndexNodeInner, VirtualIndexNodeInnerLocked, page_cache::PageCache},
    process::{kthread::spawn_kthread, timer::sleep_with_interval},
    sync::{condvar::Condvar, mutex::Mutex, spin::SpinMutex},
};

use super::{VirtualFileSystem, VirtualIndexNode};

impl VirtualFileSystem {
    fn new_inode(&self, id: u64) -> VirtualIndexNode {
        let inner = Box::new(VirtualIndexNodeInner {
            condvar: Condvar::new(),
            inner_locked: SpinMutex::new(
                VirtualIndexNodeInnerLocked {
                    inode: None,
                    strong_count: 1,
                    weak_count: 0,
                    to_destroy: false,
                    destroy_tag: 0,
                    // new_inode 只在 get_inode_with_cache 中调用，后面该 inode 必然是
                    // 会存在于 cache_inode 中的。
                    in_cache: true,
                },
                "inode_inner_locked",
            ),
            page_cache: PageCache::new(),
            id,
            destroying_lock: Mutex::new((), "inode_destroying_lock"),
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
    /// 和普通 clone 大致相同，但是不会清除 to_destroy 标记，并且不会增加 strong_count，而是
    /// 增加 weak_count
    ///
    /// 克隆出来的 [VirtualIndexNode] 不能再被 clone
    ///
    /// # Panics
    ///
    /// 如果当前 [VirtualIndexNode] 是 weak 的，则 panic
    fn weak_clone(&self) -> VirtualIndexNode {
        assert!(!self.weak);
        let inode = unsafe { self.ptr.as_ref() };
        let mut inner_locked = inode.inner_locked.lock();
        inner_locked.weak_count += 1;
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
        inner_locked.strong_count += 1;
        // inner_locked.to_destroy 如果为 true 的话，则必然是引用计数从 1 -> 2 这个 clone 过程
        // 而这个 clone 过程必然是从 inode_cache 中克隆的，此时必然是持有 inode_cache 的锁
        // 这里应该不存在和 inode 回收竞争的情况
        if inner_locked.to_destroy {
            inner_locked.to_destroy = false;
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
        if self.weak {
            inner_locked.weak_count -= 1;
        } else {
            inner_locked.strong_count -= 1;
        }
        // 如果是非 weak 引用 drop 掉，会减少 strong_count，此时如果 strong_count 为 1，则说明
        // 要准备开始释放 inode 了
        if !self.weak && inner_locked.strong_count == 1 {
            inner_locked.to_destroy = true;
            inner_locked.destroy_tag += 1;
            let destroying_id = inner_locked.destroy_tag;
            // 由于 weak clone 还要用到 inner_locked，所以这里先释放锁
            drop(inner_locked);
            let weak_inode = self.weak_clone();
            spawn_kthread(move || {
                // 有可能在销毁过程中，又出现了多次从 inode_cache 中 clone 然后又销毁的情况
                // 于是会 spawn 出多个销毁线程
                // 我们认为只有最后那个销毁线程负责完成销毁操作

                // 首先需要加锁，确保同一时间只有一个销毁线程在进行销毁
                let _destroying_lock = weak_inode.destroying_lock.lock();
                let inner_locked = weak_inode.inner_locked.lock();
                // 1. 如果 in_cache 为 false 的话，说明最后的销毁线程已经完成了销毁操作，直接返回
                // 2. 如果 destroy_tag 不等于 destroying_id 的话，说明当前线程不是最后一个销毁线程，直接返回
                // 注意，对于第二点，即使 destroy_tag 等于 destroying_id，也有可能不是最后一个销毁线程，因为
                // 有可能在后面 sync 的过程中，又有人从 inode_cache 中克隆出去了然后释放引用了，那么就会有新的销毁线程
                if !inner_locked.in_cache || inner_locked.destroy_tag != destroying_id {
                    return;
                }
                drop(inner_locked);
                // 如果在 sync 过程中又出现了新的脏页的出现，那么
                // 1. 又出现了新的引用，to_destroy 标记被清除
                // 2. to_destroy 标记被清除然后又被设置，此时一定是有了一个新的销毁线程
                weak_inode.sync();
                // TODO 这里模拟耗时操作
                sleep_with_interval(Duration::from_secs(5));
                let fs = weak_inode.fs();
                let mut inode_cache = fs.inode_cache.lock();
                // 现在对 inode_cache 加锁，可以确保 to_destroy 标记不会发生改变，
                let mut inner_locked = weak_inode.inner_locked.lock();
                // 现在对 inner_locked 加锁，可以确保 destroy_tag 不会发生改变，

                // 我们只关心 to_destroy 为 true 的情况
                // 这种情况下，当前所有的 drop 操作肯定都把 destroy_tag 设置好了，可以直接根据 destroy_tag 判定
                // 当前线程是否为最后一个销毁线程
                // 1. 只要 to_destroy 为 true，那么此时 strong_count 必然等于 1。因为第二个强引用在生成
                //    时，必然是从 inode_cache 中克隆出来的，如果克隆对 inode_cache 的加锁先于这里，那么这里再
                //    拿到锁，看到的 to_destroy 必然是为 false 的。反之，那个克隆拿到 inode_cache 的锁之后，当
                //    前 inode 实例已经被释放了
                // 2. 此时不会出现有 drop 但是还没更新 destroy_tag 的情况，因为如果有 drop，如果这个 drop 先于
                //    这里对 inner_locked 加锁的话，这里再拿到锁肯定是看到正确的 destroy_tag 了。如果这里先于
                //    那个 drop 对 inner_locked 加锁的话，to_destroy 必然是为 false 的
                if inner_locked.destroy_tag == destroying_id && inner_locked.to_destroy {
                    let cached_inode = inode_cache.remove(&weak_inode.id).unwrap();
                    inner_locked.in_cache = false;
                    drop(inner_locked);
                    // cached_inode 在 drop 时仍需要对 inner_locked 加锁，所以这里先释放 cached_inode
                    drop(cached_inode);
                }
            });
        } else if inner_locked.weak_count + inner_locked.strong_count == 0 {
            // 说明 strong_count 和 weak_count 都为 0，此时就可以安全回收内存了
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
        let inode = self.inner_locked.lock().inode();
        let id = inode.find(name)?;
        let fs = self.fs();
        Some(fs.get_inode_with_cache(id))
    }
}
