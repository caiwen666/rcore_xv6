use alloc::collections::btree_set::BTreeSet;

pub struct RecycleAllocator {
    /// 已经分配出去的 id 的数量
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
}
