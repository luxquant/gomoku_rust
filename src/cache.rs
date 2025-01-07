/// Our equivalent of the TypeScript `Cache` class.
/// It uses FIFO logic (when overflowing, we discard the oldest element).
use std::collections::HashMap;
use std::collections::VecDeque;
use std::usize;

#[derive(Clone)]
pub struct Cache<K, V> {
  capacity: usize,
  /// Store keys in a FIFO structure (VecDeque) to "shift" when overflowing
  keys_fifo: VecDeque<K>,
  /// Mapping key -> value
  map: HashMap<K, V>,
}

impl<K, V> Cache<K, V>
where
  K: std::cmp::Eq + std::hash::Hash + Clone,
{
  /// Create a new cache based on `CacheConfig`.
  pub fn new(capacity: usize) -> Self {
    let capacity = if capacity == 0 { 1_000_000 } else { capacity };
    Cache {
      capacity,
      keys_fifo: VecDeque::with_capacity(capacity),
      map: HashMap::with_capacity(capacity),
    }
  }

  /// Return the value by key (analog of `get`).
  /// If `enable_cache` == false, return None (or could be `false`,
  /// but for Rust, `Option` is preferable).
  pub fn get(&mut self, key: &K) -> Option<&V> {
    self.map.get(key)
  }

  /// Save the value (analog of `put`).
  /// - if `enable_cache == false`, do nothing.
  /// - if capacity is reached, remove the oldest key.
  pub fn put(&mut self, key: K, value: V) {
    // if the key already exists, we can update the value and not change the queue
    if self.map.contains_key(&key) {
      // update in map
      self.map.insert(key, value);
      return;
    }

    // if it didn't exist, check for overflow
    if self.keys_fifo.len() >= self.capacity {
      if let Some(oldest_key) = self.keys_fifo.pop_front() {
        self.map.remove(&oldest_key);
      }
    }

    self.keys_fifo.push_back(key.clone());
    self.map.insert(key, value);
  }

  /// Check for presence in the cache (analog of `has`).
  pub fn has(&self, key: &K) -> bool {
    self.map.contains_key(key)
  }
}
