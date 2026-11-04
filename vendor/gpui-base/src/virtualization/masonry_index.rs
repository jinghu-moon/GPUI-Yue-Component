//! O(1) index reads with two bounded directory levels for retained snapshots.
use std::{ops::Index, rc::Rc};

const BLOCK_CAPACITY: usize = 256;
const GROUP_CAPACITY: usize = 32;
type Block<T> = Rc<[T; BLOCK_CAPACITY]>;
type Group<T> = Rc<Vec<Block<T>>>;

#[derive(Clone)]
pub(super) struct IndexVector<T> {
    groups: Rc<Vec<Group<T>>>,
    len: usize,
}
impl<T> Default for IndexVector<T> {
    fn default() -> Self {
        Self {
            groups: Rc::default(),
            len: 0,
        }
    }
}
impl<T> IndexVector<T> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn get(&self, ix: usize) -> Option<&T> {
        if ix >= self.len {
            return None;
        }
        let block = ix / BLOCK_CAPACITY;
        self.groups
            .get(block / GROUP_CAPACITY)?
            .get(block % GROUP_CAPACITY)?
            .get(ix % BLOCK_CAPACITY)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.groups
            .iter()
            .flat_map(|group| group.iter())
            .flat_map(|block| block.iter())
            .take(self.len)
    }
}
impl<T: Copy + Default> IndexVector<T> {
    pub fn push(&mut self, value: T) {
        let groups = Rc::make_mut(&mut self.groups);
        if self.len.is_multiple_of(BLOCK_CAPACITY * GROUP_CAPACITY) {
            groups.push(Rc::new(Vec::new()));
        }
        let blocks = Rc::make_mut(groups.last_mut().unwrap());
        if self.len.is_multiple_of(BLOCK_CAPACITY) {
            blocks.push(Rc::new([T::default(); BLOCK_CAPACITY]));
        }
        Rc::make_mut(blocks.last_mut().unwrap())[self.len % BLOCK_CAPACITY] = value;
        self.len += 1;
    }
}
impl<T> Index<usize> for IndexVector<T> {
    type Output = T;
    fn index(&self, ix: usize) -> &T {
        self.get(ix).expect("masonry index in bounds")
    }
}
