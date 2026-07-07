use alloc::{collections::btree_set::BTreeSet, vec::Vec};

#[derive(Clone)]
pub struct RecycleAllocator<T> {
    /// 还没被分配过的最小 id
    current: usize,
    /// 回收的 id
    recycled: BTreeSet<usize>,
    items: Vec<Option<T>>,
}

impl<T> RecycleAllocator<T> {
    pub fn new() -> Self {
        Self {
            current: 0,
            recycled: BTreeSet::new(),
            items: Vec::new(),
        }
    }
    /// 将元素添加到分配器中，并返回其 id
    ///
    /// 会分配一个当前未被分配出的最小 id
    pub fn push(&mut self, item: T) -> usize {
        let id = self.recycled.pop_first().unwrap_or_else(|| {
            self.current += 1;
            self.current - 1
        });
        assert!(
            id <= self.items.len(),
            "RecycleAllocator: id {} is out of range!",
            id
        );
        if id == self.items.len() {
            self.items.push(Some(item));
        } else {
            self.items[id] = Some(item);
        }
        id
    }
    /// 从分配器中移除元素
    ///
    /// # Panics
    ///
    /// 如果对应 id 的元素不存在，则 panic
    pub fn pop(&mut self, id: usize) {
        if core::hint::unlikely(id >= self.items.len() || self.items[id].is_none()) {
            panic!("RecycleAllocator: id {} is out of range!", id);
        }
        self.recycled.insert(id);
        self.items[id] = None;
    }
    /// 直接将元素插入到某个位置，主要用于 fork。如果原来的位置有元素则会被 drop 掉
    pub fn insert(&mut self, id: usize, item: T) {
        if id >= self.items.len() {
            for i in self.items.len()..=id {
                self.items.push(None);
                self.recycled.insert(i);
            }
            self.current = id + 1;
        }
        self.recycled.remove(&id);
        self.items[id] = Some(item);
    }
    /// 获取对应 id 的元素
    ///
    /// # Returns
    ///
    /// 如果对应 id 的元素不存在，则返回 None
    pub fn get(&self, id: usize) -> Option<&T> {
        self.items.get(id).and_then(|item| item.as_ref())
    }
    /// 获取对应 id 的元素，取可变引用
    ///
    /// # Returns
    ///
    /// 如果对应 id 的元素不存在，则返回 None
    #[expect(unused)]
    pub fn get_mut(&mut self, id: usize) -> Option<&mut T> {
        self.items.get_mut(id).and_then(|item| item.as_mut())
    }
    /// 获取下一个将被分配的 id
    pub fn next_id(&self) -> usize {
        self.recycled.first().copied().unwrap_or(self.current)
    }
    /// 当前分配器中元素的数量
    pub fn len(&self) -> usize {
        self.current - self.recycled.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(id, item)| item.as_ref().map(|v| (id, v)))
    }
}
