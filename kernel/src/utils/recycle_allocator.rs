use alloc::collections::btree_set::BTreeSet;

pub struct RecycleAllocator {
    /// 还没被分配过的最小 id
    current: usize,
    /// 回收的 id
    recycled: BTreeSet<usize>,
}

impl RecycleAllocator {
    pub fn new() -> Self {
        Self {
            current: 0,
            recycled: BTreeSet::new(),
        }
    }
    pub fn alloc(&mut self) -> usize {
        if let Some(id) = self.recycled.pop_first() {
            id
        } else {
            self.current += 1;
            self.current - 1
        }
    }
    pub fn dealloc(&mut self, id: usize) {
        assert!(
            id < self.current,
            "RecycleAllocator: id {} is out of range!",
            id
        );
        assert!(
            !self.recycled.contains(&id),
            "RecycleAllocator: id {} has been deallocated!",
            id
        );
        self.recycled.insert(id);
    }
    /// 当前分配出去的数量
    pub fn count(&self) -> usize {
        self.current - self.recycled.len()
    }
}
