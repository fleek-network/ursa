use std::{cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

struct Node<T> {
    next: RefCell<Rc<Dll<T>>>,
    prev: RefCell<Rc<Dll<T>>>,
    data: T,
}

enum Dll<K> {
    _Node(Node<K>),
    _Nil,
}

struct Data<K, V> {
    value: V,
    dll: Rc<Dll<K>>,
}

struct Lru<K: Eq + Hash, V> {
    store: HashMap<K, Data<K, V>>,
    head: Rc<Dll<K>>,
    tail: Rc<Dll<K>>,
    cap: Option<usize>,
}

impl<K, V> Lru<K, V>
where
    K: Hash + Eq + Clone,
{
    pub fn _new(cap: Option<usize>) -> Self {
        let nil = Rc::new(Dll::_Nil);
        Self {
            store: if let Some(cap) = cap {
                HashMap::with_capacity(cap)
            } else {
                HashMap::new()
            },
            head: nil.clone(),
            tail: nil,
            cap,
        }
    }

    fn _get_first_key(&self) -> Option<K> {
        if let Dll::_Node(node) = self.head.as_ref() {
            Some(node.data.clone())
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
                self._remove(&first_key);
            }
        }
        let new_tail = Rc::new(Dll::_Node(Node {
            next: RefCell::new(Rc::new(Dll::_Nil)),
            prev: RefCell::new(self.tail.clone()),
            data: k.clone(),
        }));
        if let Dll::_Node(old_tail) = self.tail.as_ref() {
            *old_tail.next.borrow_mut() = new_tail.clone();
        }
        self.store.insert(
            k,
            Data {
                value: v,
                dll: new_tail.clone(),
            },
        );
        self.tail = new_tail.clone();
        if let Dll::_Nil = self.head.as_ref() {
            self.head = new_tail;
        }
    }

    pub fn _remove(&mut self, k: &K) -> Option<V> {
        self.store.remove(k).map(|data| {
            if let Dll::_Node(node) = data.dll.as_ref() {
                let prev = node.prev.borrow_mut();
                let next = node.next.borrow_mut();
                if let Dll::_Node(next) = next.as_ref() {
                    *next.prev.borrow_mut() = prev.clone();
                } else {
                    self.tail = prev.clone();
                }
                if let Dll::_Node(prev) = prev.as_ref() {
                    *prev.next.borrow_mut() = next.clone();
                } else {
                    self.head = next.clone();
                }
            }
            data.value
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<K, V> Lru<K, V>
    where
        K: Hash + Eq + Clone,
    {
        pub fn k_from_head(&self) -> Vec<K> {
            let mut items = vec![];
            let mut head = self.head.clone();
            'walk: loop {
                if let Dll::_Node(node) = head.as_ref() {
                    items.push(node.data.clone());
                    head = node.next.clone().into_inner();
                } else {
                    break 'walk;
                }
            }
            items
        }
        pub fn k_from_tail(&self) -> Vec<K> {
            let mut items = vec![];
            let mut tail = self.tail.clone();
            'walk: loop {
                if let Dll::_Node(node) = tail.as_ref() {
                    items.push(node.data.clone());
                    tail = node.prev.clone().into_inner();
                } else {
                    break 'walk;
                }
            }
            items
        }
    }

    mod no_cap {
        use super::*;

        #[test]
        fn new() {
            let lru = Lru::<&str, u8>::_new(None);
            assert_eq!(lru.cap, None);
            assert!(lru.k_from_head().is_empty());
            assert!(lru.k_from_tail().is_empty());
        }

        #[test]
        fn get_empty() {
            let lru = Lru::<&str, u8>::_new(None);
            let res = lru._get(&"a");
            assert_eq!(res, None);
        }

        #[test]
        fn remove_empty() {
            let mut lru = Lru::<&str, u8>::_new(None);
            let res = lru._remove(&"a");
            assert_eq!(res, None);
        }

        #[test]
        fn get_one() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            let res = lru._get(&"a");
            assert_eq!(res, Some(&1));
        }

        #[test]
        fn get_two() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            let res = lru._get(&"a");
            assert_eq!(res, Some(&1));
            let res = lru._get(&"b");
            assert_eq!(res, Some(&2));
        }

        #[test]
        fn get_three() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            let res = lru._get(&"a");
            assert_eq!(res, Some(&1));
            let res = lru._get(&"b");
            assert_eq!(res, Some(&2));
            let res = lru._get(&"c");
            assert_eq!(res, Some(&3));
        }

        #[test]
        fn remove_one() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._remove(&"a");
            assert!(lru.k_from_head().is_empty());
            assert!(lru.k_from_tail().is_empty());
        }

        #[test]
        fn remove_head() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._remove(&"a");
            let res = lru._get(&"a");
            assert_eq!(res, None);
            let res = lru._get(&"b");
            assert_eq!(res, Some(&2));
            assert_eq!(lru.k_from_head(), ["b"]);
            assert_eq!(lru.k_from_tail(), ["b"]);
        }

        #[test]
        fn remove_tail() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._remove(&"b");
            let res = lru._get(&"a");
            assert_eq!(res, Some(&1));
            let res = lru._get(&"b");
            assert_eq!(res, None);
            assert_eq!(lru.k_from_head(), ["a"]);
            assert_eq!(lru.k_from_tail(), ["a"]);
        }

        #[test]
        fn remove_mid() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            lru._remove(&"c");
            let res = lru._get(&"a");
            assert_eq!(res, Some(&1));
            let res = lru._get(&"b");
            assert_eq!(res, Some(&2));
            let res = lru._get(&"c");
            assert_eq!(res, None);
            assert_eq!(lru.k_from_head(), ["a", "b"]);
            assert_eq!(lru.k_from_tail(), ["b", "a"]);
        }

        #[test]
        fn insert_duplicate() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("a", 1);
            assert_eq!(lru.k_from_head(), ["a"]);
            assert_eq!(lru.k_from_tail(), ["a"]);
        }

        #[test]
        fn insert_one() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            assert_eq!(lru.k_from_head(), ["a"]);
            assert_eq!(lru.k_from_tail(), ["a"]);
        }

        #[test]
        fn insert_two() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            assert_eq!(lru.k_from_head(), ["a", "b"]);
            assert_eq!(lru.k_from_tail(), ["b", "a"]);
        }

        #[test]
        fn insert_three() {
            let mut lru = Lru::_new(None);
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            assert_eq!(lru.k_from_head(), ["a", "b", "c"]);
            assert_eq!(lru.k_from_tail(), ["c", "b", "a"]);
        }
    }

    mod cap {
        use super::*;

        #[test]
        fn new() {
            let lru = Lru::<&str, u8>::_new(Some(3));
            assert_eq!(lru.cap, Some(3));
            assert!(lru.k_from_head().is_empty());
            assert!(lru.k_from_tail().is_empty());
        }

        #[test]
        fn insert_exceed_cap() {
            let mut lru = Lru::_new(Some(3));
            lru._insert("a", 1);
            lru._insert("b", 2);
            lru._insert("c", 3);
            lru._insert("d", 4);
            assert_eq!(lru.k_from_head(), ["b", "c", "d"]);
            assert_eq!(lru.k_from_tail(), ["d", "c", "b"]);
            let res = lru._get(&"a");
            assert_eq!(res, None);
            let res = lru._get(&"b");
            assert_eq!(res, Some(&2));
            let res = lru._get(&"c");
            assert_eq!(res, Some(&3));
            let res = lru._get(&"d");
            assert_eq!(res, Some(&4));
        }
    }
}
