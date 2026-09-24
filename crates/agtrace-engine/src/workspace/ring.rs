//! Fixed-capacity ring buffer used for bounded live state (feed, per-agent timeline).

use std::collections::VecDeque;

/// Bounded FIFO: pushing onto a full buffer drops the oldest element.
///
/// `N` is the capacity. [`RingBuffer::pushed`] counts every push ever made, so a
/// consumer can tell how many entries were appended (and dropped) since it last looked.
#[derive(Debug, Clone, PartialEq)]
pub struct RingBuffer<T, const N: usize> {
    items: VecDeque<T>,
    pushed: u64,
}

impl<T, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub const CAPACITY: usize = N;

    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
            pushed: 0,
        }
    }

    /// Append an element, dropping the oldest one when full.
    pub fn push(&mut self, item: T) {
        if N == 0 {
            return;
        }
        if self.items.len() == N {
            self.items.pop_front();
        }
        self.items.push_back(item);
        self.pushed += 1;
    }

    /// Insert keeping the buffer ordered by `key` (after existing equal keys).
    /// When full, the smallest element is dropped (possibly the new one).
    pub fn insert_by_key<K: Ord>(&mut self, item: T, key: impl Fn(&T) -> K) {
        if N == 0 {
            return;
        }
        let k = key(&item);
        let pos = self.items.partition_point(|x| key(x) <= k);
        self.pushed += 1;
        if self.items.len() == N {
            if pos == 0 {
                return;
            }
            self.items.pop_front();
            self.items.insert(pos - 1, item);
        } else {
            self.items.insert(pos, item);
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn capacity(&self) -> usize {
        N
    }

    /// Total number of pushes since creation (or the last [`RingBuffer::clear`]).
    pub fn pushed(&self) -> u64 {
        self.pushed
    }

    /// Oldest to newest.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> + ExactSizeIterator {
        self.items.iter_mut()
    }

    pub fn last(&self) -> Option<&T> {
        self.items.back()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.pushed = 0;
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a RingBuffer<T, N> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_oldest_when_full() {
        let mut r: RingBuffer<u32, 3> = RingBuffer::new();
        for i in 0..5 {
            r.push(i);
        }
        assert_eq!(r.len(), 3);
        assert_eq!(r.iter().copied().collect::<Vec<_>>(), vec![2, 3, 4]);
        assert_eq!(r.pushed(), 5);
        assert_eq!(r.last(), Some(&4));
        assert_eq!(r.capacity(), 3);
    }

    #[test]
    fn insert_by_key_keeps_order_and_drops_smallest() {
        let mut r: RingBuffer<(u32, char), 3> = RingBuffer::new();
        r.insert_by_key((5, 'a'), |x| x.0);
        r.insert_by_key((1, 'b'), |x| x.0);
        r.insert_by_key((5, 'c'), |x| x.0);
        assert_eq!(
            r.iter().copied().collect::<Vec<_>>(),
            vec![(1, 'b'), (5, 'a'), (5, 'c')]
        );
        // Full: a new middle element evicts the smallest.
        r.insert_by_key((3, 'd'), |x| x.0);
        assert_eq!(r.iter().map(|x| x.1).collect::<String>(), "dac".to_string());
        // Full and older than everything: dropped.
        r.insert_by_key((0, 'e'), |x| x.0);
        assert_eq!(r.iter().map(|x| x.1).collect::<String>(), "dac");
    }

    #[test]
    fn clear_resets_counter() {
        let mut r: RingBuffer<u32, 2> = RingBuffer::new();
        r.push(1);
        r.clear();
        assert!(r.is_empty());
        assert_eq!(r.pushed(), 0);
    }

    #[test]
    fn zero_capacity_is_inert() {
        let mut r: RingBuffer<u32, 0> = RingBuffer::new();
        r.push(1);
        assert!(r.is_empty());
    }
}
