use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const SHARD_COUNT: usize = 4;

type Entries = HashMap<String, (Instant, String)>;

pub struct QueryCache {
    shards: [Mutex<Entries>; SHARD_COUNT],
    default_ttl_secs: u64,
    shutdown: AtomicBool,
}

impl Drop for QueryCache {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl QueryCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            shards: std::array::from_fn(|_| Mutex::new(HashMap::new())),
            default_ttl_secs: ttl_secs,
            shutdown: AtomicBool::new(false),
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
        let mut guard = self.shards[idx].lock().ok()?;
        let (ts, val) = guard.get(key)?;
        if ts.elapsed().as_secs() < self.default_ttl_secs {
            Some(val.clone())
        } else {
            // P0: 删除过期条目，避免 HashMap 无限增长
            guard.remove(key);
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

    // P0: 后台定期清理过期条目（每 5 分钟一次）
    pub fn spawn_cleanup_task(self: &Arc<Self>) {
        let cache = Arc::clone(self);
        let ttl = self.default_ttl_secs;
        std::thread::spawn(move || {
            while !cache.shutdown.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(300));
                if cache.shutdown.load(Ordering::Relaxed) {
                    break;
                }
                for shard in &cache.shards {
                    if let Ok(mut guard) = shard.lock() {
                        guard.retain(|_, (ts, _)| ts.elapsed().as_secs() < ttl);
                    }
                }
            }
        });
    }
}
