use std::{marker::PhantomData, ptr::NonNull};

use rand::Rng;

struct Node<K, V> {
    key: std::mem::MaybeUninit<K>,
    value: std::mem::MaybeUninit<V>,
    level: usize,
    next: Vec<Option<NonNull<Node<K, V>>>>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V, level: usize, max_level: usize) -> Self {
        Self {
            key: std::mem::MaybeUninit::new(key),
            value: std::mem::MaybeUninit::new(value),
            level,
            next: vec![None; max_level],
        }
    }

    fn sigil(max_level: usize) -> Self {
        Self {
            key: std::mem::MaybeUninit::uninit(),
            value: std::mem::MaybeUninit::uninit(),
            level: 0,
            next: vec![None; max_level],
        }
    }
}

pub struct SkipList<K, V> {
    head: NonNull<Node<K, V>>,
    level: usize,
    max_level: usize,
    marker: PhantomData<Node<K, V>>,
}

impl<K, V> Default for SkipList<K, V> {
    fn default() -> Self {
        let max_level = 12;
        let node = Box::leak(Box::new(Node::sigil(max_level))).into();
        Self {
            head: node,
            level: 0,
            max_level,
            marker: PhantomData,
        }
    }
}

impl<K: Ord, V> SkipList<K, V> {
    pub fn get(&self, k: &K) -> Option<&V> {
        let mut node = self.head;
        for l in (0..self.level).rev() {
            unsafe {
                while let Some(next) = node.as_ref().next[l] {
                    let key = &*next.as_ref().key.as_ptr();
                    if key == k {
                        return Some(&*next.as_ref().value.as_ptr());
                    }
                    if key < k {
                        node = next;
                    } else {
                        break;
                    }
                }
            }
        }
        None
    }

    pub fn insert(&mut self, k: K, mut v: V) -> Option<V> {
        let mut node = self.head;
        let mut updates = vec![None; self.max_level];

        for l in (0..self.level).rev() {
            unsafe {
                while let Some(mut next) = node.as_ref().next[l] {
                    let key = &*next.as_ref().key.as_ptr();
                    if key == &k {
                        let value = &mut *next.as_mut().value.as_mut_ptr();
                        std::mem::swap(value, &mut v);
                        return Some(v);
                    }
                    if key < &k {
                        node = next;
                    } else {
                        break;
                    }
                }
            }
            updates[l] = Some(node);
        }

        let level = self.random_level();
        if level > self.level {
            for node in updates.iter_mut().take(level).skip(self.level) {
                node.replace(self.head);
            }
            self.level = level;
        }

        let mut node: NonNull<Node<K, V>> =
            Box::leak(Box::new(Node::new(k, v, level, self.max_level))).into();

        for (l, ln) in updates.iter_mut().enumerate().take(level) {
            if let Some(ln) = ln {
                unsafe {
                    node.as_mut().next[l] = ln.as_ref().next[l];
                    ln.as_mut().next[l] = Some(node);
                }
            }
        }
        None
    }

    pub fn delete(&mut self, k: &K) -> Option<V> {
        let mut node = self.head;
        let mut updates = vec![None; self.max_level];

        let mut target = None;
        for l in (0..self.level).rev() {
            unsafe {
                while let Some(next) = node.as_ref().next[l] {
                    let key = &*next.as_ref().key.as_ptr();
                    if key == k {
                        target = Some(next);
                        break;
                    }
                    if key < k {
                        node = next;
                    } else {
                        break;
                    }
                }
            }
            updates[l] = Some(node);
        }

        if let Some(node) = target {
            unsafe {
                for (l, ln) in updates.iter().enumerate().take(node.as_ref().level) {
                    if let Some(mut ln) = ln {
                        ln.as_mut().next[l] = node.as_ref().next[l];
                    }
                }

                let node = Box::from_raw(node.as_ptr());
                return Some(node.value.assume_init());
            }
        }
        None
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..self.max_level)
    }
}

impl<K, V> Drop for SkipList<K, V> {
    fn drop(&mut self) {
        unsafe {
            let mut node = self.head.as_mut().next[0];

            while let Some(n) = node {
                let mut n = Box::from_raw(n.as_ptr());
                node = n.next[0];
                n.key.assume_init_drop();
                n.value.assume_init_drop();
            }

            Box::from_raw(self.head.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SkipList;
    #[test]
    fn test_of_skip_list() {
        let mut skip_list = SkipList::default();
        for i in 0..100 {
            assert_eq!(skip_list.insert(i, i), None);
        }

        for i in 0..100 {
            assert_eq!(skip_list.insert(i, 10 * i), Some(i));
        }

        for i in 0..100 {
            let v = i * 10;
            assert_eq!(skip_list.get(&i), Some(&v))
        }

        for i in 0..50 {
            let v = 10 * i;
            assert_eq!(skip_list.delete(&i), Some(v));
        }

        for i in 0..50 {
            assert_eq!(skip_list.get(&i), None);
        }

        for i in 50..100 {
            let v = i * 10;
            assert_eq!(skip_list.get(&i), Some(&v));
        }
    }
}
