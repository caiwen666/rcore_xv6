use alloc::{collections::btree_map::BTreeMap, vec::Vec};

struct LRUNode<K, V> {
    pub prev: Option<usize>,
    pub next: Option<usize>,
    pub data: Option<(K, V)>,
}

pub struct LRU<K, V> {
    cached_blocks: Vec<LRUNode<K, V>>,
    cached_index: BTreeMap<K, usize>,
    // free list 为单链表，里面的 node 的 prev 为 None
    free_head: Option<usize>,
    // used list 为双链表
    used_head: Option<usize>,
    used_tail: Option<usize>,
}

impl<K: Ord + Clone, V> LRU<K, V> {
    /// - `cache_size`：最大缓存数量
    pub fn new(cache_size: usize) -> Self {
        let mut cached_blocks = Vec::with_capacity(cache_size);
        for i in 0..cache_size - 1 {
            cached_blocks.push(LRUNode {
                prev: None,
                next: Some(i + 1),
                data: None,
            });
        }
        cached_blocks.push(LRUNode {
            prev: None,
            next: None,
            data: None,
        });
        Self {
            cached_blocks,
            cached_index: BTreeMap::new(),
            free_head: Some(0),
            used_head: None,
            used_tail: None,
        }
    }

    /// 把节点从 used list 中移除，不释放节点数据
    fn remove_used(&mut self, index: usize) {
        assert!(self.cached_blocks[index].data.is_some());
        if self.used_head == Some(index) {
            self.used_head = self.cached_blocks[index].next;
        }
        if self.used_tail == Some(index) {
            self.used_tail = self.cached_blocks[index].prev;
        }
        if let Some(prev) = self.cached_blocks[index].prev {
            self.cached_blocks[prev].next = self.cached_blocks[index].next;
        }
        if let Some(next) = self.cached_blocks[index].next {
            self.cached_blocks[next].prev = self.cached_blocks[index].prev;
        }
    }

    /// 把节点添加到 used list 结尾
    fn push_used(&mut self, index: usize) {
        let node = &mut self.cached_blocks[index];
        assert!(node.data.is_some());
        node.prev = self.used_tail;
        node.next = None;
        if let Some(used_tail) = self.used_tail {
            self.cached_blocks[used_tail].next = Some(index);
        }
        self.used_tail = Some(index);
        if self.used_head.is_none() {
            self.used_head = Some(index);
        }
    }

    /// 从 free list 中获取一个节点
    fn pop_free(&mut self) -> Option<usize> {
        if let Some(free_head) = self.free_head {
            let node = &mut self.cached_blocks[free_head];
            assert!(node.data.is_none());
            self.free_head = node.next;
            Some(free_head)
        } else {
            None
        }
    }

    /// 获取缓存值，如果缓存不存在，则调用 f 获取值并缓存
    pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> V) -> &mut V {
        if let Some(index) = self.cached_index.get(&key) {
            let index = *index;
            self.remove_used(index);
            self.push_used(index);
            let (_, value) = self.cached_blocks[index]
                .data
                .as_mut()
                .expect("data is none");
            value
        } else {
            if let Some(free_head) = self.pop_free() {
                self.cached_blocks[free_head].data = Some((key.clone(), f()));
                self.push_used(free_head);
                self.cached_index.insert(key, free_head);
                let (_, value) = self.cached_blocks[free_head]
                    .data
                    .as_mut()
                    .expect("data is none");
                value
            } else {
                let index = self.used_head.expect("used list is empty");
                let (old_key, _) = self.cached_blocks[index]
                    .data
                    .as_ref()
                    .expect("data is none");
                self.cached_index.remove(old_key);
                self.remove_used(index);
                self.cached_blocks[index].data = Some((key.clone(), f()));
                self.push_used(index);
                self.cached_index.insert(key, index);
                let (_, value) = self.cached_blocks[index]
                    .data
                    .as_mut()
                    .expect("data is none");
                value
            }
        }
    }
}
