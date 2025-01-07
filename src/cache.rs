/// Structure for describing cache settings (capacity and whether the cache is enabled).
/// In the TS version, this could be `CONFIG.ENABLE_CACHE`.
/// Here we make the `enable_cache` parameter independent.
pub struct CacheConfig {
  pub capacity: usize,
  pub enable_cache: bool,
}

impl Default for CacheConfig {
  fn default() -> Self {
    // Default values: capacity = 1_000_000, enable_cache = true
    CacheConfig {
      capacity: 1_000_000,
      enable_cache: true,
    }
  }
}

/// Our equivalent of the TypeScript `Cache` class.
/// It uses FIFO logic (when overflowing, we discard the oldest element).
use std::collections::HashMap;
use std::collections::VecDeque;

pub struct Cache<K, V> {
  /// Cache settings
  config: CacheConfig,
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
  pub fn new(config: CacheConfig) -> Self {
    let capacity = config.capacity;
    Cache {
      config,
      keys_fifo: VecDeque::with_capacity(capacity),
      map: HashMap::with_capacity(capacity),
    }
  }

  /// Return the value by key (analog of `get`).
  /// If `enable_cache` == false, return None (or could be `false`,
  /// but for Rust, `Option` is preferable).
  pub fn get(&mut self, key: &K) -> Option<&V> {
    // If caching is disabled, return None
    if !self.config.enable_cache {
      return None;
    }
    self.map.get(key)
  }

  /// Save the value (analog of `put`).
  /// - if `enable_cache == false`, do nothing.
  /// - if capacity is reached, remove the oldest key.
  pub fn put(&mut self, key: K, value: V) {
    if !self.config.enable_cache {
      return;
    }

    // if the key already exists, we can update the value and not change the queue
    if self.map.contains_key(&key) {
      // update in map
      self.map.insert(key, value);
      return;
    }

    // if it didn't exist, check for overflow
    if self.keys_fifo.len() >= self.config.capacity {
      if let Some(oldest_key) = self.keys_fifo.pop_front() {
        self.map.remove(&oldest_key);
      }
    }

    self.keys_fifo.push_back(key.clone());
    self.map.insert(key, value);
  }

  /// Check for presence in the cache (analog of `has`).
  pub fn has(&self, key: &K) -> bool {
    if !self.config.enable_cache {
      return false;
    }
    self.map.contains_key(key)
  }
}
