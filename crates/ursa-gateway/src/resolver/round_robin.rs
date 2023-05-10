use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, RwLock, TryLockError},
    time::Duration,
};
use tokio::time::Instant;

const TIME_SHARING_DURATION: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub struct Queue<T> {
    inner: Arc<RwLock<Inner<T>>>,
}

#[derive(Debug)]
struct Inner<T> {
    addresses: VecDeque<T>,
    stamp: Instant,
}

impl<T> Queue<T>
where
    T: Clone + PartialEq,
{
    pub fn new(addresses: HashSet<T>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                addresses: addresses.into_iter().collect(),
                stamp: Instant::now(),
            })),
        }
    }

    pub fn next(&self) -> Option<T> {
        let inner = self.inner.read().unwrap();
        let now = Instant::now();
        if now.duration_since(inner.stamp) >= TIME_SHARING_DURATION {
            drop(inner);
            match self.inner.try_write() {
                Ok(mut writer_guard) => {
                    let front = writer_guard.addresses.pop_front()?;
                    writer_guard.addresses.push_back(front.clone());
                    writer_guard.stamp = now;
                    Some(front)
                }
                Err(TryLockError::WouldBlock) => {
                    self.inner.read().unwrap().addresses.front().cloned()
                }
                Err(TryLockError::Poisoned(e)) => panic!("{e}"),
            }
        } else {
            inner.addresses.front().cloned()
        }
    }

    pub fn remove(&self, addr: T) {
        let mut inner = self.inner.write().unwrap();
        inner.stamp = Instant::now();
        inner.addresses.retain(|a| a != &addr);
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().addresses.is_empty()
    }
}
