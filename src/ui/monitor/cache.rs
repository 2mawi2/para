use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
struct CacheEntry {
    activity_time: Option<DateTime<Utc>>,
    cached_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ActivityCache {
    entries: Arc<Mutex<HashMap<PathBuf, CacheEntry>>>,
    ttl: Duration,
}

impl ActivityCache {
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::seconds(ttl_seconds),
        }
    }

    pub fn get(&self, path: &PathBuf) -> Option<Option<DateTime<Utc>>> {
        match self.entries.lock() {
            Ok(entries) => {
                if let Some(entry) = entries.get(path) {
                    let age = Utc::now() - entry.cached_at;
                    if age < self.ttl {
                        return Some(entry.activity_time);
                    }
                }
                None
            }
            Err(_) => {
                // Mutex poisoned - return cache miss
                None
            }
        }
    }

    pub fn set(&self, path: PathBuf, activity_time: Option<DateTime<Utc>>) {
        // Try to update cache, but don't fail if mutex is poisoned
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(
                path,
                CacheEntry {
                    activity_time,
                    cached_at: Utc::now(),
                },
            );
        }
        // If mutex is poisoned, silently ignore - caching is not critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_cache_basic_operations() {
        let cache = ActivityCache::new(2); // 2 second TTL
        let path = PathBuf::from("/test/path");
        let time = Utc::now();

        // Test set and get
        cache.set(path.clone(), Some(time));
        assert_eq!(cache.get(&path), Some(Some(time)));

        // Test cache miss
        let other_path = PathBuf::from("/other/path");
        assert_eq!(cache.get(&other_path), None);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = ActivityCache::new(1); // 1 second TTL
        let path = PathBuf::from("/test/path");
        let time = Utc::now();

        cache.set(path.clone(), Some(time));
        assert_eq!(cache.get(&path), Some(Some(time)));

        // Wait for cache to expire
        thread::sleep(StdDuration::from_millis(1100));
        assert_eq!(cache.get(&path), None);
    }

    #[test]
    fn test_cache_none_values() {
        let cache = ActivityCache::new(5);
        let path = PathBuf::from("/test/path");

        // Test caching None values
        cache.set(path.clone(), None);
        assert_eq!(cache.get(&path), Some(None));
    }

    #[test]
    fn test_cache_update_existing() {
        let cache = ActivityCache::new(5);
        let path = PathBuf::from("/test/path");
        let time1 = Utc::now();
        let time2 = time1 + chrono::Duration::hours(1);

        // Set initial value
        cache.set(path.clone(), Some(time1));
        assert_eq!(cache.get(&path), Some(Some(time1)));

        // Update with new value
        cache.set(path.clone(), Some(time2));
        assert_eq!(cache.get(&path), Some(Some(time2)));
    }

    #[test]
    fn test_cache_thread_safety() {
        use std::sync::Arc;

        let cache = Arc::new(ActivityCache::new(5));
        let path = PathBuf::from("/test/path");
        let time = Utc::now();

        // Spawn multiple threads that read and write
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let path_clone = path.clone();
            let thread_time = time + chrono::Duration::milliseconds(i * 100);

            let handle = thread::spawn(move || {
                // Write
                cache_clone.set(path_clone.clone(), Some(thread_time));
                thread::sleep(StdDuration::from_millis(10));

                // Read
                let result = cache_clone.get(&path_clone);
                assert!(result.is_some());
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Final value should be cached
        assert!(cache.get(&path).is_some());
    }

    #[test]
    fn test_cache_multiple_paths() {
        let cache = ActivityCache::new(5);
        let paths: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("/test/path{}", i)))
            .collect();
        let base_time = Utc::now();

        // Set different times for different paths
        for (i, path) in paths.iter().enumerate() {
            let time = base_time + chrono::Duration::seconds(i as i64);
            cache.set(path.clone(), Some(time));
        }

        // Verify all are cached correctly
        for (i, path) in paths.iter().enumerate() {
            let expected_time = base_time + chrono::Duration::seconds(i as i64);
            assert_eq!(cache.get(path), Some(Some(expected_time)));
        }
    }

    #[test]
    fn test_cache_ttl_boundary() {
        let cache = ActivityCache::new(2); // 2 second TTL for more reliable testing
        let path = PathBuf::from("/test/path");
        let time = Utc::now();

        cache.set(path.clone(), Some(time));

        // Well before expiration (50% of TTL)
        thread::sleep(StdDuration::from_millis(1000));
        assert_eq!(
            cache.get(&path),
            Some(Some(time)),
            "Should still be cached at 1000ms (50% of TTL)"
        );

        // After expiration
        thread::sleep(StdDuration::from_millis(1500));
        assert_eq!(cache.get(&path), None, "Should be expired at 2500ms");
    }
}
