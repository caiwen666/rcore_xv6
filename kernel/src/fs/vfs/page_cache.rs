use crate::{
    arch::MMArch,
    fs::vfs::{VirtualIndexNode, interface::FileType},
    mm::{
        MemoryManagementArch,
        allocator::{PageFrame, alloc_frame},
    },
    sync::{
        mutex::{Mutex, MutexGuard},
        spin::SpinMutex,
    },
    utils::BlockIterator,
};
use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    sync::Arc,
};

pub struct PageCache {
    table: SpinMutex<BTreeMap<usize, Arc<Mutex<CachedPage>>>>,
    /// 尚未处理的 dirty page
    dirty: SpinMutex<BTreeSet<usize>>,
    /// 正在写回 dirty page、正在加载 page、正在 resize，
    /// 这三种操作都需要持有这个锁，在这个锁上同步。
    /// 因此有一个性质：持有该锁期间，文件的大小不会被改变。
    page_lock: Mutex<()>,
}

pub struct CachedPage {
    frame: Option<PageFrame>,
}

impl PageCache {
    pub fn new() -> Self {
        Self {
            table: SpinMutex::new(BTreeMap::new(), "page_cache_table"),
            dirty: SpinMutex::new(BTreeSet::new(), "page_cache_dirty"),
            page_lock: Mutex::new((), "page_cache_page_lock"),
        }
    }
}

impl VirtualIndexNode {
    /// [CachedPage::frame] 可能为 None，这说明 [CachedPage] 对应的页帧可能尚未加载或是
    /// 之前加载了但是被换出，此时就需要调用本函数。
    ///
    /// 本函数会分配物理页并加载数据到其中，调用本函数后，[CachedPage::frame] 必定不为 None。
    fn prepare_page<'a>(
        &self,
        offset: usize,
        mut cached_page: MutexGuard<'a, CachedPage>,
    ) -> MutexGuard<'a, CachedPage> {
        assert!(offset.is_multiple_of(MMArch::PAGE_SIZE));
        assert!(cached_page.frame.is_none());
        let _page_lock = self.page_cache.page_lock.lock();
        let inode = self.inner_locked.lock().inode();
        // 有可能文件大小被改变了，当前这个 offset 超出了文件大小
        // 但是此时走到这里，我们必须给一个 page frame，
        // 因此这种情况下我们只给个 page frame，但是不写数据
        let mut frame = alloc_frame(1).unwrap();
        if offset < inode.metadata().size {
            let end = (offset + MMArch::PAGE_SIZE).min(inode.metadata().size) - offset;
            // 保证读取数据在文件大小范围内
            inode.read_at(offset, &mut frame.as_mut_slice()[..end]);
        }
        cached_page.frame = Some(frame);
        cached_page
    }
    fn get_page(&self, offset: usize) -> Arc<Mutex<CachedPage>> {
        assert!(offset.is_multiple_of(MMArch::PAGE_SIZE));
        let mut table = self.page_cache.table.lock();
        let cached_page = table
            .entry(offset)
            .or_insert_with(|| Arc::new(Mutex::new(CachedPage { frame: None }, "cached_page")));
        cached_page.clone()
    }
}

impl VirtualIndexNode {
    /// 在 inode 的指定偏移量开始，读取指定大小的数据
    ///
    /// 如果读取到的数据长度小于 `buf` 的长度，则 `buf` 剩余部分不会被修改。
    ///
    /// 该函数会首先获取当前文件快照的文件大小，然后按这个文件大小读数据，
    /// 如果中间文件被扩容了，也只能最多读取到扩容前的文件大小。如果中间
    /// 文件被缩容了，那么也会读到缩容前的文件大小，但是多出来的数据会是
    /// 脏数据。
    ///
    /// **会堵塞**
    ///
    /// # Returns
    ///
    /// 返回成功读取的字节数
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let metadata = self.inner_locked.lock().inode().metadata();
        if offset >= metadata.size {
            return 0;
        }
        let size = buf.len().min(metadata.size - offset);
        let mut pos = 0;
        for block in BlockIterator::new(MMArch::PAGE_SIZE, offset, size) {
            let cached_page = self.get_page(block.block_id() * MMArch::PAGE_SIZE);
            let mut page_guard = cached_page.lock();
            let frame = if let Some(frame) = page_guard.frame.as_mut() {
                frame
            } else {
                page_guard = self.prepare_page(block.block_id() * MMArch::PAGE_SIZE, page_guard);
                page_guard.frame.as_mut().unwrap()
            };
            let data = frame.as_mut_slice();
            buf[pos..pos + block.size()]
                .copy_from_slice(&data[block.offset()..block.offset() + block.size()]);
            pos += block.size();
        }
        pos
    }

    /// 在 inode 的指定偏移量开始，写入指定大小的数据
    ///
    /// 如果写入到的数据长度小于 `buf` 的长度，则 `buf` 剩余部分不会被修改。
    ///
    /// 该函数会首先获取当前文件快照的文件大小，然后按这个文件大小写数据，
    /// 如果中间文件被扩容了，也只能最多写入到扩容前的文件大小。如果中间
    /// 文件被缩容了，那么也会写入到缩容前的文件大小，但是多出来的数据会
    /// 被写入到页缓存，如果后面再扩容的话有机会被落盘，否则会丢失。
    ///
    /// **会堵塞**
    ///
    /// # Returns
    ///
    /// 返回成功写入的字节数
    #[expect(unused)]
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let metadata = self.inner_locked.lock().inode().metadata();
        if offset >= metadata.size {
            return 0;
        }
        let size = buf.len().min(metadata.size - offset);
        let mut pos = 0;
        for block in BlockIterator::new(MMArch::PAGE_SIZE, offset, size) {
            let file_offset = block.block_id() * MMArch::PAGE_SIZE;
            let cached_page = self.get_page(file_offset);
            let mut page_guard = cached_page.lock();
            let frame = if let Some(frame) = page_guard.frame.as_mut() {
                frame
            } else {
                page_guard = self.prepare_page(file_offset, page_guard);
                page_guard.frame.as_mut().unwrap()
            };
            let data = frame.as_mut_slice();
            data[block.offset()..block.offset() + block.size()]
                .copy_from_slice(&buf[pos..pos + block.size()]);
            self.page_cache.dirty.lock().insert(file_offset);
            pos += block.size();
        }
        pos
    }

    /// 将 inode 中的全部 page cache 写回
    ///
    /// 如果上一个 sync 还没完成，该函数会等待上一个 sync 完成
    ///
    /// 该函数只会将本函数**开始工作时**的**瞬间**的 dirty page 写回，如果
    /// 在此期间有新的 dirty page 产生，则这些新的 dirty page 不会被写回。
    ///
    /// **会阻塞**
    pub fn sync(&self) {
        let inode = self.inner_locked.lock().inode();
        let mut working_dirty = BTreeSet::new();
        let _page_lock = self.page_cache.page_lock.lock();
        let mut dirty = self.page_cache.dirty.lock();
        working_dirty.extend(dirty.iter().cloned());
        dirty.clear();
        drop(dirty);
        let file_size = inode.metadata().size;
        for offset in working_dirty {
            // 尽管 resize 和 sync 是互斥的，但是往 dirty 里面添加元素时不会加
            // page_lock 锁，所以有可能存在超出文件大小的 dirty page
            if offset >= file_size {
                continue;
            }
            // 也有可能存在没超过文件大小的 dirty page，但是不存在 table 中
            // 比如文件大小先缩小，于是 table 中的一些 dirty page 被直接丢弃，但是 offset 还在 dirty 中
            // 然后文件大小再增大，table 中的新页面还没加载，于是就有这个情况
            let Some(cached_page) = self.page_cache.table.lock().get(&offset).cloned() else {
                continue;
            };
            let end = (offset + MMArch::PAGE_SIZE).min(file_size) - offset;
            let mut page_guard = cached_page.lock();
            let frame = page_guard.frame.as_mut().unwrap();
            inode.write_at(offset, &frame.as_mut_slice()[..end]);
        }
    }

    /// 更改文件大小
    ///
    /// **会堵塞**
    ///
    /// # Panics
    ///
    /// - 如果当前 inode 的类型不是文件，则 panic
    #[expect(unused)]
    pub fn resize(&self, new_size: usize) {
        // 这里也拿 writing_back 锁，这样就能保证 resize 和 sync 不会同时进行
        let _page_lock = self.page_cache.page_lock.lock();
        let inode = self.inner_locked.lock().inode();
        if inode.metadata().file_type != FileType::File {
            panic!("resize called on non-file inode");
        }
        inode.resize(new_size);
        // 移除超出文件大小的 page cache
        // 有可能被删除的 Arc 还是存在引用，但这个我们就没办法了，
        // 为了高并发，只能牺牲点一致性
        self.page_cache.table.lock().split_off(&new_size);
        // 不用再把 dirty 中超出文件大小的 offset 移除
        // 因为当前可能仍存在超出文件大小的 page cache 的 Arc 引用，这些引用在 write
        // 结束后会把 offset 加到 dirty，而往 dirty 里面添加元素是不需要加 page_lock
        // 的。
    }
}
