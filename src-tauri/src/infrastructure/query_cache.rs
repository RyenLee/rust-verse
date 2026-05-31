use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

const SHARD_COUNT: usize = 4;

type Entries = HashMap<String, (Instant, String)>;

pub struct QueryCache {
    shards: [Mutex<Entries>; SHARD_COUNT],
    default_ttl_secs: u64,
}

impl QueryCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            shards: std::array::from_fn(|_| Mutex::new(HashMap::new())),
            default_ttl_secs: ttl_secs,
        }
    }

    fn shard_index(key: &str) -> usize {
        let mut hash: u64 = 5381;
        for b in key.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(b as u64);
        }
        (hash as usize) % SHARD_COUNT
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let idx = Self::shard_index(key);
        let guard = self.shards[idx].lock().ok()?;
        let (ts, val) = guard.get(key)?;
        if ts.elapsed().as_secs() < self.default_ttl_secs {
            Some(val.clone())
        } else {
            None
        }
    }

    pub fn set(&self, key: String, value: String) {
        let idx = Self::shard_index(&key);
        if let Ok(mut guard) = self.shards[idx].lock() {
            guard.insert(key, (Instant::now(), value));
        }
    }

    pub fn invalidate_all(&self) {
        for shard in &self.shards {
            if let Ok(mut guard) = shard.lock() {
                guard.clear();
            }
        }
    }
}