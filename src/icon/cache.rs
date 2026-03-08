use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::fs;
use std::io::Write;

use bevy::prelude::Handle;
use bevy::render::texture::Image;
use lru::LruCache;
use serde::{Deserialize, Serialize};

use crate::icon::types::IconError;

/// Cached icon entry in memory
#[derive(Debug, Clone)]
pub struct CachedIcon {
    /// Bevy image handle for the loaded icon
    pub handle: Handle<Image>,
    /// Path to the icon file
    pub path: PathBuf,
    /// When this icon was last accessed
    pub last_used: Instant,
    /// When this icon was first cached
    pub cached_at: Instant,
    /// Number of times this icon has been accessed
    pub access_count: u64,
}

impl CachedIcon {
    /// Create a new cached icon entry
    pub fn new(handle: Handle<Image>, path: PathBuf) -> Self {
        let now = Instant::now();
        Self {
            handle,
            path,
            last_used: now,
            cached_at: now,
            access_count: 1,
        }
    }

    /// Mark this icon as accessed
    pub fn mark_accessed(&mut self) {
        self.last_used = Instant::now();
        self.access_count += 1;
    }

    /// Get the age of this cached entry
    pub fn age(&self) -> Duration {
        self.cached_at.elapsed()
    }

    /// Get time since last access
    pub fn time_since_last_access(&self) -> Duration {
        self.last_used.elapsed()
    }
}

/// Persistent cache entry for disk storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistentCacheEntry {
    /// Path to the icon file
    path: PathBuf,
    /// When this entry was created
    created_at: u64, // Unix timestamp
    /// Number of times this has been used
    usage_count: u64,
    /// Last time this was accessed
    last_accessed: u64, // Unix timestamp
}

impl PersistentCacheEntry {
    fn new(path: PathBuf) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            path,
            created_at: now,
            usage_count: 1,
            last_accessed: now,
        }
    }

    fn mark_accessed(&mut self) {
        self.usage_count += 1;
        self.last_accessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    fn age_seconds(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(self.created_at)
    }
}

/// Cache metrics for monitoring performance
#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Number of entries evicted from memory cache
    pub evictions: u64,
    /// Number of persistent cache saves
    pub persistent_saves: u64,
    /// Number of persistent cache loads
    pub persistent_loads: u64,
    /// Number of cleanup operations performed
    pub cleanups: u64,
}

impl CacheMetrics {
    /// Calculate hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Get total number of requests
    pub fn total_requests(&self) -> u64 {
        self.hits + self.misses
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// Configuration for the icon cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of icons to keep in memory
    pub memory_cache_size: usize,
    /// Whether to enable persistent cache
    pub enable_persistent_cache: bool,
    /// Path to persistent cache file
    pub persistent_cache_path: PathBuf,
    /// Maximum age for persistent cache entries (in seconds)
    pub max_persistent_age: u64,
    /// Maximum number of entries in persistent cache
    pub max_persistent_entries: usize,
    /// How often to perform cleanup (in seconds)
    pub cleanup_interval: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("anny-dock");
        
        Self {
            memory_cache_size: 100,
            enable_persistent_cache: true,
            persistent_cache_path: cache_dir.join("icon_cache.json"),
            max_persistent_age: 7 * 24 * 60 * 60, // 7 days
            max_persistent_entries: 1000,
            cleanup_interval: 60 * 60, // 1 hour
        }
    }
}

/// Icon cache with LRU memory cache and persistent storage
pub struct IconCache {
    /// In-memory LRU cache of loaded icons
    memory_cache: LruCache<String, CachedIcon>,
    /// Persistent cache mapping class names to icon paths
    persistent_cache: HashMap<String, PersistentCacheEntry>,
    /// Cache configuration
    config: CacheConfig,
    /// Performance metrics
    metrics: CacheMetrics,
    /// Last time cleanup was performed
    last_cleanup: Instant,
    /// Whether persistent cache has been modified since last save
    persistent_dirty: bool,
}

impl IconCache {
    /// Create a new icon cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new icon cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        let memory_cache = LruCache::new(
            std::num::NonZeroUsize::new(config.memory_cache_size)
                .unwrap_or(std::num::NonZeroUsize::new(100).unwrap())
        );

        let mut cache = Self {
            memory_cache,
            persistent_cache: HashMap::new(),
            config,
            metrics: CacheMetrics::default(),
            last_cleanup: Instant::now(),
            persistent_dirty: false,
        };

        // Load persistent cache if enabled
        if cache.config.enable_persistent_cache {
            if let Err(e) = cache.load_persistent_cache() {
                eprintln!("Warning: Failed to load persistent icon cache: {}", e);
            }
        }

        cache
    }

    /// Get an icon from cache by class name
    pub fn get(&mut self, class: &str) -> Option<&CachedIcon> {
        // Check memory cache first
        if let Some(cached) = self.memory_cache.get_mut(class) {
            cached.mark_accessed();
            self.metrics.hits += 1;
            return Some(cached);
        }

        self.metrics.misses += 1;
        None
    }

    /// Get icon path from persistent cache
    pub fn get_persistent_path(&mut self, class: &str) -> Option<PathBuf> {
        if !self.config.enable_persistent_cache {
            return None;
        }

        if let Some(entry) = self.persistent_cache.get_mut(class) {
            // Check if file still exists
            if entry.path.exists() {
                entry.mark_accessed();
                self.persistent_dirty = true;
                self.metrics.persistent_loads += 1;
                return Some(entry.path.clone());
            } else {
                // File no longer exists, remove from cache
                self.persistent_cache.remove(class);
                self.persistent_dirty = true;
            }
        }

        None
    }

    /// Store an icon in the cache
    pub fn store(&mut self, class: String, icon: CachedIcon) {
        // Store in memory cache
        if let Some(_evicted) = self.memory_cache.put(class.clone(), icon.clone()) {
            self.metrics.evictions += 1;
        }

        // Store in persistent cache if enabled
        if self.config.enable_persistent_cache {
            let entry = PersistentCacheEntry::new(icon.path.clone());
            self.persistent_cache.insert(class, entry);
            self.persistent_dirty = true;
        }

        // Perform cleanup if needed
        self.maybe_cleanup();
    }

    /// Store only the path mapping in persistent cache
    pub fn store_path_mapping(&mut self, class: String, path: PathBuf) {
        if !self.config.enable_persistent_cache {
            return;
        }

        let entry = PersistentCacheEntry::new(path);
        self.persistent_cache.insert(class, entry);
        self.persistent_dirty = true;
        self.maybe_cleanup();
    }

    /// Remove an entry from all caches
    pub fn remove(&mut self, class: &str) {
        self.memory_cache.pop(class);
        if self.config.enable_persistent_cache {
            if self.persistent_cache.remove(class).is_some() {
                self.persistent_dirty = true;
            }
        }
    }

    /// Clear all caches
    pub fn clear(&mut self) {
        self.memory_cache.clear();
        if self.config.enable_persistent_cache {
            if !self.persistent_cache.is_empty() {
                self.persistent_cache.clear();
                self.persistent_dirty = true;
            }
        }
        self.metrics.reset();
    }

    /// Get current cache metrics
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            memory_entries: self.memory_cache.len(),
            memory_capacity: self.memory_cache.cap().get(),
            persistent_entries: self.persistent_cache.len(),
            hit_rate: self.metrics.hit_rate(),
            total_requests: self.metrics.total_requests(),
        }
    }

    /// Force cleanup of old entries
    pub fn cleanup(&mut self) -> Result<(), IconError> {
        self.cleanup_persistent_cache()?;
        self.save_persistent_cache()?;
        self.metrics.cleanups += 1;
        self.last_cleanup = Instant::now();
        Ok(())
    }

    /// Check if cleanup should be performed and do it if needed
    fn maybe_cleanup(&mut self) {
        let cleanup_interval = Duration::from_secs(self.config.cleanup_interval);
        if self.last_cleanup.elapsed() >= cleanup_interval {
            if let Err(e) = self.cleanup() {
                eprintln!("Warning: Cache cleanup failed: {}", e);
            }
        }
    }

    /// Load persistent cache from disk
    fn load_persistent_cache(&mut self) -> Result<(), IconError> {
        if !self.config.persistent_cache_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.config.persistent_cache_path)
            .map_err(|e| IconError::cache_error(format!("Failed to read persistent cache: {}", e)))?;

        let cache: HashMap<String, PersistentCacheEntry> = serde_json::from_str(&content)
            .map_err(|e| IconError::cache_error(format!("Failed to parse persistent cache: {}", e)))?;

        self.persistent_cache = cache;
        self.persistent_dirty = false;
        Ok(())
    }

    /// Save persistent cache to disk
    fn save_persistent_cache(&mut self) -> Result<(), IconError> {
        if !self.config.enable_persistent_cache || !self.persistent_dirty {
            return Ok(());
        }

        // Ensure cache directory exists
        if let Some(parent) = self.config.persistent_cache_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| IconError::cache_error(format!("Failed to create cache directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(&self.persistent_cache)
            .map_err(|e| IconError::cache_error(format!("Failed to serialize cache: {}", e)))?;

        let mut file = fs::File::create(&self.config.persistent_cache_path)
            .map_err(|e| IconError::cache_error(format!("Failed to create cache file: {}", e)))?;

        file.write_all(content.as_bytes())
            .map_err(|e| IconError::cache_error(format!("Failed to write cache file: {}", e)))?;

        self.persistent_dirty = false;
        self.metrics.persistent_saves += 1;
        Ok(())
    }

    /// Clean up old entries from persistent cache
    fn cleanup_persistent_cache(&mut self) -> Result<(), IconError> {
        if !self.config.enable_persistent_cache {
            return Ok(());
        }

        let max_age = self.config.max_persistent_age;
        let max_entries = self.config.max_persistent_entries;
        let mut removed_count = 0;

        // Remove entries that are too old or have invalid paths
        self.persistent_cache.retain(|_class, entry| {
            if entry.age_seconds() > max_age || !entry.path.exists() {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        // If still too many entries, remove least recently used
        if self.persistent_cache.len() > max_entries {
            let mut entries: Vec<_> = self.persistent_cache.iter().collect();
            entries.sort_by_key(|(_, entry)| entry.last_accessed);
            
            let to_remove = self.persistent_cache.len() - max_entries;
            let keys_to_remove: Vec<String> = entries.iter()
                .take(to_remove)
                .map(|(class, _)| (*class).clone())
                .collect();
            
            for class in keys_to_remove {
                self.persistent_cache.remove(&class);
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            self.persistent_dirty = true;
        }

        Ok(())
    }
}

impl Drop for IconCache {
    fn drop(&mut self) {
        // Save persistent cache on drop
        if let Err(e) = self.save_persistent_cache() {
            eprintln!("Warning: Failed to save persistent cache on drop: {}", e);
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in memory cache
    pub memory_entries: usize,
    /// Maximum capacity of memory cache
    pub memory_capacity: usize,
    /// Number of entries in persistent cache
    pub persistent_entries: usize,
    /// Cache hit rate as percentage
    pub hit_rate: f64,
    /// Total number of cache requests
    pub total_requests: u64,
}

impl CacheStats {
    /// Get memory cache utilization as percentage
    pub fn memory_utilization(&self) -> f64 {
        if self.memory_capacity == 0 {
            0.0
        } else {
            (self.memory_entries as f64 / self.memory_capacity as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> (CacheConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            memory_cache_size: 5,
            enable_persistent_cache: true,
            persistent_cache_path: temp_dir.path().join("test_cache.json"),
            max_persistent_age: 3600, // 1 hour
            max_persistent_entries: 10,
            cleanup_interval: 60,
        };
        (config, temp_dir)
    }

    fn create_test_icon() -> CachedIcon {
        CachedIcon::new(
            Handle::default(),
            PathBuf::from("/test/icon.png")
        )
    }

    #[test]
    fn test_cache_creation() {
        let cache = IconCache::new();
        assert_eq!(cache.stats().memory_entries, 0);
        assert_eq!(cache.stats().persistent_entries, 0);
        assert_eq!(cache.metrics().total_requests(), 0);
    }

    #[test]
    fn test_memory_cache_store_and_get() {
        let mut cache = IconCache::new();
        let icon = create_test_icon();
        
        // Store icon
        cache.store("test-app".to_string(), icon.clone());
        
        // Should be able to retrieve it
        let retrieved = cache.get("test-app");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().path, icon.path);
        
        // Should be a cache hit
        assert_eq!(cache.metrics().hits, 1);
        assert_eq!(cache.metrics().misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = IconCache::new();
        
        // Try to get non-existent icon
        let result = cache.get("non-existent");
        assert!(result.is_none());
        
        // Should be a cache miss
        assert_eq!(cache.metrics().hits, 0);
        assert_eq!(cache.metrics().misses, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let (config, _temp_dir) = create_test_config();
        let mut cache = IconCache::with_config(config);
        
        // Fill cache beyond capacity
        for i in 0..10 {
            let icon = CachedIcon::new(
                Handle::default(),
                PathBuf::from(format!("/test/icon{}.png", i))
            );
            cache.store(format!("app{}", i), icon);
        }
        
        // Memory cache should be at capacity
        assert_eq!(cache.stats().memory_entries, 5);
        assert!(cache.metrics().evictions > 0);
        
        // First few entries should be evicted
        assert!(cache.get("app0").is_none());
        assert!(cache.get("app1").is_none());
        
        // Last entries should still be there
        assert!(cache.get("app9").is_some());
        assert!(cache.get("app8").is_some());
    }

    #[test]
    fn test_persistent_cache() {
        let (config, _temp_dir) = create_test_config();
        let mut cache = IconCache::with_config(config.clone());
        
        // Store path mapping
        cache.store_path_mapping("test-app".to_string(), PathBuf::from("/test/icon.png"));
        
        // Should be able to retrieve from persistent cache
        let path = cache.get_persistent_path("test-app");
        assert!(path.is_none()); // File doesn't actually exist
        
        // Create a real temporary file
        let temp_file = _temp_dir.path().join("real_icon.png");
        fs::write(&temp_file, b"fake icon data").unwrap();
        
        cache.store_path_mapping("real-app".to_string(), temp_file.clone());
        let path = cache.get_persistent_path("real-app");
        assert_eq!(path, Some(temp_file));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = IconCache::new();
        let icon = create_test_icon();
        
        // Store some data
        cache.store("test-app".to_string(), icon);
        assert_eq!(cache.stats().memory_entries, 1);
        
        // Clear cache
        cache.clear();
        assert_eq!(cache.stats().memory_entries, 0);
        assert_eq!(cache.metrics().total_requests(), 0);
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = IconCache::new();
        let icon = create_test_icon();
        
        // Store icon
        cache.store("test-app".to_string(), icon);
        assert!(cache.get("test-app").is_some());
        
        // Remove icon
        cache.remove("test-app");
        assert!(cache.get("test-app").is_none());
    }

    #[test]
    fn test_cache_metrics() {
        let mut cache = IconCache::new();
        let icon = create_test_icon();
        
        // Initial state
        assert_eq!(cache.metrics().hit_rate(), 0.0);
        
        // Store and access
        cache.store("test-app".to_string(), icon);
        cache.get("test-app"); // hit
        cache.get("non-existent"); // miss
        
        let metrics = cache.metrics();
        assert_eq!(metrics.hits, 1);
        assert_eq!(metrics.misses, 1);
        assert_eq!(metrics.hit_rate(), 50.0);
        assert_eq!(metrics.total_requests(), 2);
    }

    #[test]
    fn test_cached_icon_access_tracking() {
        let mut icon = create_test_icon();
        let initial_count = icon.access_count;
        let initial_time = icon.last_used;
        
        // Wait a bit to ensure time difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        icon.mark_accessed();
        
        assert_eq!(icon.access_count, initial_count + 1);
        assert!(icon.last_used > initial_time);
        assert!(icon.age() > Duration::from_millis(0));
    }

    #[test]
    fn test_cache_stats() {
        let (config, _temp_dir) = create_test_config();
        let mut cache = IconCache::with_config(config);
        
        let stats = cache.stats();
        assert_eq!(stats.memory_entries, 0);
        assert_eq!(stats.memory_capacity, 5);
        assert_eq!(stats.memory_utilization(), 0.0);
        
        // Add some entries
        for i in 0..3 {
            let icon = create_test_icon();
            cache.store(format!("app{}", i), icon);
        }
        
        let stats = cache.stats();
        assert_eq!(stats.memory_entries, 3);
        assert_eq!(stats.memory_utilization(), 60.0); // 3/5 * 100
    }
}