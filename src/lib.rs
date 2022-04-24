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
    len: usize,
    level: usize,
    max_level: usize,
    marker: PhantomData<Node<K, V>>,
}

pub struct Iter<'a, K: 'a, V: 'a> {
    len: usize,
    head: Option<NonNull<Node<K, V>>>,
    marker: PhantomData<&'a Node<K, V>>,
}

pub struct IterMut<'a, K: 'a, V: 'a> {
    len: usize,
    head: Option<NonNull<Node<K, V>>>,
    marker: PhantomData<&'a Node<K, V>>,
}

pub struct IntoIter<K, V> {
    len: usize,
    head: Option<NonNull<Node<K, V>>>,
    marker: PhantomData<Node<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.head.map(|node| unsafe {
            self.head = node.as_ref().next[0];
            self.len -= 1;
            let k = node.as_ref().key.assume_init_ref();
            let v = node.as_ref().value.assume_init_ref();
            (k, v)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.head.map(|mut node| unsafe {
            self.head = node.as_ref().next[0];
            self.len -= 1;
            let k = node.as_ref().key.assume_init_ref();
            let v = &mut *node.as_mut().value.as_mut_ptr();
            (k, v)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.head.map(|node| unsafe {
            let node = Box::from_raw(node.as_ptr());

            self.head = node.next[0];
            self.len -= 1;
            let k = node.key.assume_init();
            let v = node.value.assume_init();

            (k, v)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len
    }
}

impl<K, V> Default for SkipList<K, V> {
    fn default() -> Self {
        let max_level = 12;
        let node = Box::leak(Box::new(Node::sigil(max_level))).into();
        Self {
            head: node,
            len: 0,
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
        self.len += 1;
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
                self.len -= 1;
                let mut node = Box::from_raw(node.as_ptr());
                node.key.assume_init_drop();
                return Some(node.value.assume_init());
            }
        }
        None
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            len: self.len,
            head: unsafe { self.head.as_ref().next[0] },
            marker: PhantomData,
        }
    }

    pub fn iter_mut(&self) -> IterMut<'_, K, V> {
        IterMut {
            len: self.len,
            head: unsafe { self.head.as_ref().next[0] },
            marker: PhantomData,
        }
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..self.max_level)
    }
}

impl<K, V> IntoIterator for SkipList<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(mut self) -> Self::IntoIter {
        let node = unsafe { self.head.as_ref().next[0] };
        unsafe {
            self.head.as_mut().next[0] = None;
        }
        IntoIter {
            len: self.len,
            head: node,
            marker: PhantomData,
        }
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

        for (k, v) in skip_list.iter_mut() {
            *v = *k * 20;
        }

        let mut key = 50;
        for (k, v) in skip_list.iter() {
            assert_eq!(*k, key);
            assert_eq!(*v, key * 20);
            key += 1;
        }

        key = 50;
        for (k, v) in skip_list.into_iter() {
            assert_eq!(k, key);
            assert_eq!(v, key * 20);
            key += 1;
        }
    }
}
