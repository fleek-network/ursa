use std::{cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

struct _Node<T> {
    next: RefCell<Rc<_Dll<T>>>,
    prev: RefCell<Rc<_Dll<T>>>,
    data: Rc<T>,
}

enum _Dll<T> {
    _Node(_Node<T>),
    _Nil,
}

struct _Data<K, V> {
    value: V,
    dll: Rc<_Dll<K>>,
}

struct _Lru<K, V> {
    store: HashMap<Rc<K>, _Data<K, V>>,
    head: Rc<_Dll<K>>,
    tail: Rc<_Dll<K>>,
    cap: Option<usize>,
}

impl<K, V> _Lru<K, V>
where
    K: Hash + Eq + Clone,
{
    pub fn _new(cap: Option<usize>) -> Self {
        let nil = Rc::new(_Dll::_Nil);
        Self {
            store: if let Some(cap) = cap {
                HashMap::with_capacity(cap)
            } else {
                HashMap::new()
            },
            head: Rc::clone(&nil),
            tail: nil,
            cap,
        }
    }

    fn _get_first_key(&self) -> Option<Rc<K>> {
        if let _Dll::_Node(node) = self.head.as_ref() {
            Some(Rc::clone(&node.data))
        } else {
            None
        }
    }

    fn _contains(&self, k: &K) -> bool {
        self.store.contains_key(k)
    }

    pub fn _get(&self, k: &K) -> Option<&V> {
        self.store.get(k).map(|data| &data.value)
    }

    pub fn _insert(&mut self, k: K, v: V) {
        if self._contains(&k) {
            return;
        }
        if let Some(cap) = self.cap {
            if cap <= self.store.len() {
                let first_key = self
                    ._get_first_key()
                    .expect("[LRU]: Failed to get the first key while deleting.");
                self._remove(first_key.as_ref());
            }
        }
        let key = Rc::new(k);
        let new_tail = Rc::new(_Dll::_Node(_Node {
            next: RefCell::new(Rc::new(_Dll::_Nil)),
            prev: RefCell::new(Rc::clone(&self.tail)),
            data: Rc::clone(&key),
        }));
        if let _Dll::_Node(old_tail) = self.tail.as_ref() {
            *old_tail.next.borrow_mut() = Rc::clone(&new_tail);
        }
        self.store.insert(
            key,
            _Data {
                value: v,
                dll: Rc::clone(&new_tail),
            },
        );
        self.tail = Rc::clone(&new_tail);
        if let _Dll::_Nil = self.head.as_ref() {
            self.head = new_tail;
        }
    }

    pub fn _remove(&mut self, k: &K) -> Option<V> {
        self.store.remove(k).map(|data| {
            if let _Dll::_Node(node) = data.dll.as_ref() {
                let prev = node.prev.borrow_mut();
                let next = node.next.borrow_mut();
                if let _Dll::_Node(next) = next.as_ref() {
                    *next.prev.borrow_mut() = Rc::clone(&prev);
                } else {
                    self.tail = Rc::clone(&prev);
                }
                if let _Dll::_Node(prev) = prev.as_ref() {
                    *prev.next.borrow_mut() = Rc::clone(&next);
                } else {
                    self.head = Rc::clone(&next);
                }
            }
            data.value
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<K, V> _Lru<K, V>
    where
        K: Hash + Eq + Clone,
    {
        pub fn k_from_head(&self) -> Vec<Rc<K>> {
            let mut items = vec![];
            let mut head = Rc::clone(&self.head);
            'walk: loop {
                if let _Dll::_Node(node) = head.as_ref() {
                    items.push(Rc::clone(&node.data));
                    head = node.next.clone().into_inner(); // RefCell.clone
                } else {
                    break 'walk;
                }
            }
            items
        }
        pub fn k_from_tail(&self) -> Vec<Rc<K>> {
            let mut items = vec![];
            let mut tail = Rc::clone(&self.tail);
            'walk: loop {
                if let _Dll::_Node(node) = tail.as_ref() {
                    items.push(Rc::clone(&node.data));
                    tail = node.prev.clone().into_inner(); // RefCell.clone
                } else {
                    break 'walk;
                }
            }
            items
        }
    }

    pub fn ref_to_k<K: Clone>(vec: Vec<Rc<K>>) -> Vec<K> {
        vec.into_iter().map(|k| k.as_ref().clone()).collect()
    }

    mod no_cap {
        use super::*;

        #[test]
        fn new() {
            let lru = _Lru::<&str, u8>::_new(None);
            assert_eq!(lru.cap, None);
            assert!(lru.k_from_head().is_empty());
            assert!(lru.k_from_tail().is_empty());
        }

        #[test]
        fn get_empty() {
            let lru = _Lru::<&str, u8>::_new(None);
            let res = lru._get(&"a");
            assert_eq!(res, None);
        }

        #[test]
        fn remove_empty() {
            let mut lru = _Lru::<&str, u8>::_new(None);
            let res = lru._remove(&"a");
            assert_eq!(res, None);
        }

        #[test]
        fn get_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            assert_eq!(lru._get(&"a"), Some(&1));
        }

        #[test]
        fn get_two() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
        }

        #[test]
        fn get_three() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(lru._get(&"c"), Some(&3));
        }

        #[test]
        fn remove_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._remove(&"a");
            assert!(lru.k_from_head().is_empty());
            assert!(lru.k_from_tail().is_empty());
        }

        #[test]
        fn remove_head() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._remove(&"a");
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(ref_to_k(lru.k_from_head()), ["b"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["b"]);
        }

        #[test]
        fn remove_tail() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._remove(&"b");
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(ref_to_k(lru.k_from_head()), ["a"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["a"]);
        }

        #[test]
        fn remove_mid() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            lru._remove(&"b");
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(lru._get(&"c"), Some(&3));
            assert_eq!(ref_to_k(lru.k_from_head()), ["a", "c"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["c", "a"]);
        }

        #[test]
        fn insert_duplicate() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("a", 1);
            assert_eq!(ref_to_k(lru.k_from_head()), ["a"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["a"]);
        }

        #[test]
        fn insert_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            assert_eq!(ref_to_k(lru.k_from_head()), ["a"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["a"]);
        }

        #[test]
        fn insert_two() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            assert_eq!(ref_to_k(lru.k_from_head()), ["a", "b"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["b", "a"]);
        }

        #[test]
        fn insert_three() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            assert_eq!(ref_to_k(lru.k_from_head()), ["a", "b", "c"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["c", "b", "a"]);
        }
    }

    mod cap {
        use super::*;

        #[test]
        fn new() {
            let lru = _Lru::<&str, u8>::_new(Some(3));
            assert_eq!(lru.cap, Some(3));
            assert!(lru.k_from_head().is_empty());
            assert!(lru.k_from_tail().is_empty());
        }

        #[test]
        fn insert_exceed_cap() {
            let mut lru = _Lru::_new(Some(3));
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            lru._insert("d", 4);
            assert_eq!(ref_to_k(lru.k_from_head()), ["b", "c", "d"]);
            assert_eq!(ref_to_k(lru.k_from_tail()), ["d", "c", "b"]);
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(lru._get(&"c"), Some(&3));
            assert_eq!(lru._get(&"d"), Some(&4));
        }
    }
}
