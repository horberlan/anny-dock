use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, warn, error};

use crate::icon::types::{IconContext, IconResult, IconMetadata, IconFormat};
use crate::icon::traits::IconDetectionStrategy;

/// Strategy that searches standard icon directories for icons
/// 
/// This strategy performs direct filesystem searches in standard icon directories
/// like /usr/share/icons, /usr/share/pixmaps, ~/.local/share/icons, etc.
/// It supports multiple icon formats (PNG, SVG, XPM) and implements caching
/// to avoid repeated filesystem traversals.
pub struct DirectoryStrategy {
    /// Standard icon directories to search
    search_directories: Vec<PathBuf>,
    /// Cache of directory contents to avoid repeated filesystem scans
    directory_cache: Arc<RwLock<HashMap<PathBuf, DirectoryCache>>>,
    /// Cache TTL - how long directory cache entries remain valid
    cache_ttl: Duration,
    /// Supported icon formats in order of preference
    supported_formats: Vec<IconFormat>,
    /// Maximum recursion depth for directory traversal
    max_depth: usize,
}

/// Cached directory information
#[derive(Debug, Clone)]
struct DirectoryCache {
    /// Map of icon names (without extension) to full paths
    icons: HashMap<String, Vec<PathBuf>>,
    /// When this cache entry was created
    created_at: Instant,
    /// Whether this directory was successfully scanned
    scan_successful: bool,
}

impl DirectoryCache {
    fn new() -> Self {
        Self {
            icons: HashMap::new(),
            created_at: Instant::now(),
            scan_successful: false,
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

impl DirectoryStrategy {
    /// Create a new DirectoryStrategy with default settings
    pub fn new() -> Self {
        Self {
            search_directories: Self::default_search_directories(),
            directory_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            supported_formats: vec![
                IconFormat::Svg,  // Prefer vector formats
                IconFormat::Png,  // Then high-quality raster
                IconFormat::Xpm,  // Legacy format
            ],
            max_depth: 4, // Reasonable depth to avoid infinite recursion
        }
    }

    /// Create a DirectoryStrategy with custom directories
    pub fn with_directories(directories: Vec<PathBuf>) -> Self {
        let mut strategy = Self::new();
        strategy.search_directories = directories;
        strategy
    }

    /// Create a DirectoryStrategy with custom cache TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Create a DirectoryStrategy with custom max depth
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Add additional search directories
    pub fn add_directory(&mut self, directory: PathBuf) {
        if !self.search_directories.contains(&directory) {
            self.search_directories.push(directory);
        }
    }

    /// Get the default icon search directories
    fn default_search_directories() -> Vec<PathBuf> {
        let mut directories = Vec::new();

        // System-wide icon directories
        directories.push(PathBuf::from("/usr/share/icons"));
        directories.push(PathBuf::from("/usr/share/pixmaps"));
        directories.push(PathBuf::from("/usr/local/share/icons"));
        directories.push(PathBuf::from("/usr/local/share/pixmaps"));

        // User-specific directories
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            directories.push(home_path.join(".local/share/icons"));
            directories.push(home_path.join(".icons"));
        }

        // XDG data directories
        if let Some(xdg_data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
            for dir in std::env::split_paths(&xdg_data_dirs) {
                directories.push(dir.join("icons"));
                directories.push(dir.join("pixmaps"));
            }
        }

        // Flatpak directories
        directories.push(PathBuf::from("/var/lib/flatpak/exports/share/icons"));
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            directories.push(home_path.join(".local/share/flatpak/exports/share/icons"));
        }

        // Filter to only existing directories
        directories.into_iter()
            .filter(|dir| dir.exists() && dir.is_dir())
            .collect()
    }

    /// Search for an icon in all configured directories
    fn search_icon(&self, icon_name: &str) -> Option<PathBuf> {
        debug!("DirectoryStrategy: Searching for icon '{}'", icon_name);

        for directory in &self.search_directories {
            if let Some(path) = self.search_in_directory(directory, icon_name) {
                debug!("DirectoryStrategy: Found icon '{}' at {:?}", icon_name, path);
                return Some(path);
            }
        }

        debug!("DirectoryStrategy: Icon '{}' not found in any directory", icon_name);
        None
    }

    /// Search for an icon in a specific directory
    fn search_in_directory(&self, directory: &Path, icon_name: &str) -> Option<PathBuf> {
        // Check cache first
        if let Some(cached_path) = self.check_cache(directory, icon_name) {
            return Some(cached_path);
        }

        // If not in cache or cache is expired, scan the directory
        self.scan_directory(directory);

        // Check cache again after scanning
        self.check_cache(directory, icon_name)
    }

    /// Check if an icon is in the directory cache
    fn check_cache(&self, directory: &Path, icon_name: &str) -> Option<PathBuf> {
        let cache = match self.directory_cache.read() {
            Ok(cache) => cache,
            Err(e) => {
                error!("DirectoryStrategy: Failed to acquire cache read lock: {}", e);
                return None;
            }
        };

        if let Some(dir_cache) = cache.get(directory) {
            if !dir_cache.is_expired(self.cache_ttl) && dir_cache.scan_successful {
                if let Some(paths) = dir_cache.icons.get(icon_name) {
                    // Return the first path with a preferred format
                    for format in &self.supported_formats {
                        for path in paths {
                            if let Some(ext) = path.extension() {
                                if let Some(ext_str) = ext.to_str() {
                                    if IconFormat::from_extension(ext_str) == *format {
                                        return Some(path.clone());
                                    }
                                }
                            }
                        }
                    }
                    // If no preferred format found, return the first available
                    return paths.first().cloned();
                }
            }
        }

        None
    }

    /// Scan a directory and update the cache
    fn scan_directory(&self, directory: &Path) {
        let mut cache = match self.directory_cache.write() {
            Ok(cache) => cache,
            Err(e) => {
                error!("DirectoryStrategy: Failed to acquire cache write lock: {}", e);
                return;
            }
        };

        // Check if we need to scan (not in cache or expired)
        let needs_scan = cache.get(directory)
            .map(|dir_cache| dir_cache.is_expired(self.cache_ttl))
            .unwrap_or(true);

        if !needs_scan {
            return;
        }

        debug!("DirectoryStrategy: Scanning directory {:?}", directory);

        let mut dir_cache = DirectoryCache::new();
        
        match self.scan_directory_recursive(directory, 0, &mut dir_cache.icons) {
            Ok(()) => {
                dir_cache.scan_successful = true;
                debug!("DirectoryStrategy: Successfully scanned {:?}, found {} icons", 
                       directory, dir_cache.icons.len());
                

            }
            Err(e) => {
                warn!("DirectoryStrategy: Failed to scan directory {:?}: {}", directory, e);
                dir_cache.scan_successful = false;
            }
        }

        cache.insert(directory.to_path_buf(), dir_cache);
    }

    /// Recursively scan a directory for icon files
    fn scan_directory_recursive(
        &self,
        directory: &Path,
        current_depth: usize,
        icons: &mut HashMap<String, Vec<PathBuf>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if current_depth >= self.max_depth {
            return Ok(());
        }

        let entries = fs::read_dir(directory)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                if let Err(e) = self.scan_directory_recursive(&path, current_depth + 1, icons) {
                    debug!("DirectoryStrategy: Failed to scan subdirectory {:?}: {}", path, e);
                    // Continue with other directories even if one fails
                }
            } else if path.is_file() {
                // Check if this is an icon file
                if let Some(icon_name) = self.extract_icon_name(&path) {
                    icons.entry(icon_name)
                        .or_insert_with(Vec::new)
                        .push(path);
                }
            }
        }

        Ok(())
    }

    /// Extract icon name from a file path if it's a supported icon format
    fn extract_icon_name(&self, path: &Path) -> Option<String> {
        let extension = path.extension()?.to_str()?;
        let format = IconFormat::from_extension(extension);

        // Check if this is a supported format
        let is_supported = self.supported_formats.contains(&format) || 
                          matches!(format, IconFormat::Other(_));

        if is_supported {
            path.file_stem()?.to_str().map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Generate possible icon names from the context
    fn generate_icon_names(&self, context: &IconContext) -> Vec<String> {
        let mut names = Vec::new();

        // Primary name: exact class name
        names.push(context.class.clone());

        // Lowercase version
        let lowercase_class = context.class.to_lowercase();
        if lowercase_class != context.class {
            names.push(lowercase_class.clone());
        }

        // Replace common separators with hyphens
        let hyphenated = lowercase_class.replace('_', "-").replace('.', "-");
        if hyphenated != lowercase_class {
            names.push(hyphenated);
        }

        // Try without common prefixes/suffixes
        let cleaned = lowercase_class
            .trim_start_matches("org.")
            .trim_start_matches("com.")
            .trim_start_matches("net.")
            .trim_end_matches(".desktop")
            .to_string();
        if cleaned != lowercase_class && !names.contains(&cleaned) {
            names.push(cleaned);
        }

        // If we have an executable name, try that too
        if let Some(ref executable) = context.executable {
            let exec_lower = executable.to_lowercase();
            if !names.contains(&exec_lower) {
                names.push(exec_lower);
            }
        }

        // Try extracting application name from title if available
        if let Some(ref title) = context.title {
            // Common patterns: "AppName - Document", "Document - AppName", etc.
            let title_lower = title.to_lowercase();
            
            // Try splitting on common separators and take the first/last part
            for separator in &[" - ", " – ", " | ", ": "] {
                if let Some(parts) = title_lower.split(separator).collect::<Vec<_>>().get(0) {
                    let app_name = parts.trim().to_string();
                    if !app_name.is_empty() && !names.contains(&app_name) {
                        names.push(app_name);
                    }
                }
            }
        }

        debug!("DirectoryStrategy: Generated icon names for '{}': {:?}", 
               context.class, names);
        names
    }

    /// Clear the directory cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.directory_cache.write() {
            cache.clear();
            debug!("DirectoryStrategy: Cache cleared");
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Option<(usize, usize)> {
        if let Ok(cache) = self.directory_cache.read() {
            let total_entries = cache.len();
            let expired_entries = cache.values()
                .filter(|entry| entry.is_expired(self.cache_ttl))
                .count();
            Some((total_entries, expired_entries))
        } else {
            None
        }
    }
}

impl IconDetectionStrategy for DirectoryStrategy {
    fn detect_icon(&self, context: &IconContext) -> Option<IconResult> {
        let icon_names = self.generate_icon_names(context);

        for icon_name in icon_names {
            if let Some(path) = self.search_icon(&icon_name) {
                // Determine the format from the file extension
                let format = path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(IconFormat::from_extension)
                    .unwrap_or(IconFormat::Other("unknown".to_string()));

                let metadata = IconMetadata::new(format);

                return Some(IconResult::new(
                    path,
                    "DirectoryStrategy".to_string(),
                    0.7, // Medium confidence - we found a file but can't verify it's correct
                    metadata,
                ));
            }
        }

        None
    }

    fn priority(&self) -> u8 {
        25 // Medium-low priority - fallback after more specific strategies
    }

    fn name(&self) -> &'static str {
        "DirectoryStrategy"
    }

    fn is_available(&self) -> bool {
        // Available if we have at least one search directory
        !self.search_directories.is_empty()
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("DirectoryStrategy: Initializing with {} search directories", 
               self.search_directories.len());
        
        for dir in &self.search_directories {
            debug!("DirectoryStrategy: Will search in {:?}", dir);
        }

        Ok(())
    }

    fn cleanup(&mut self) {
        self.clear_cache();
        debug!("DirectoryStrategy: Cleanup completed");
    }
}

impl Default for DirectoryStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_icon_structure() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let icons_dir = temp_dir.path().join("icons");
        
        // Create directory structure
        fs::create_dir_all(&icons_dir).unwrap();
        fs::create_dir_all(icons_dir.join("hicolor/48x48/apps")).unwrap();
        fs::create_dir_all(icons_dir.join("Adwaita/scalable/apps")).unwrap();
        
        // Create test icon files
        fs::write(icons_dir.join("firefox.png"), b"fake png").unwrap();
        fs::write(icons_dir.join("chrome.svg"), b"fake svg").unwrap();
        fs::write(icons_dir.join("hicolor/48x48/apps/gimp.png"), b"fake png").unwrap();
        fs::write(icons_dir.join("Adwaita/scalable/apps/nautilus.svg"), b"fake svg").unwrap();
        
        temp_dir
    }

    #[test]
    fn test_directory_strategy_creation() {
        let strategy = DirectoryStrategy::new();
        assert_eq!(strategy.name(), "DirectoryStrategy");
        assert_eq!(strategy.priority(), 25);
        assert!(strategy.is_available()); // Should have some default directories
    }

    #[test]
    fn test_custom_directories() {
        let custom_dirs = vec![PathBuf::from("/custom/path")];
        let strategy = DirectoryStrategy::with_directories(custom_dirs.clone());
        assert_eq!(strategy.search_directories, custom_dirs);
    }

    #[test]
    fn test_icon_name_generation() {
        let strategy = DirectoryStrategy::new();
        
        let context = IconContext::new("org.mozilla.Firefox".to_string())
            .with_executable("firefox".to_string());
        
        let names = strategy.generate_icon_names(&context);
        
        assert!(names.contains(&"org.mozilla.Firefox".to_string()));
        assert!(names.contains(&"org.mozilla.firefox".to_string()));
        assert!(names.contains(&"org-mozilla-firefox".to_string()));
        assert!(names.contains(&"mozilla.firefox".to_string()));
        assert!(names.contains(&"firefox".to_string()));
    }

    #[test]
    fn test_icon_name_from_title() {
        let strategy = DirectoryStrategy::new();
        
        let context = IconContext::with_title("unknown".to_string(), "Firefox - Mozilla Firefox".to_string());
        
        let names = strategy.generate_icon_names(&context);
        
        assert!(names.contains(&"firefox".to_string()));
    }

    #[test]
    fn test_extract_icon_name() {
        let strategy = DirectoryStrategy::new();
        
        assert_eq!(
            strategy.extract_icon_name(&PathBuf::from("/path/to/firefox.png")),
            Some("firefox".to_string())
        );
        
        assert_eq!(
            strategy.extract_icon_name(&PathBuf::from("/path/to/app.svg")),
            Some("app".to_string())
        );
        
        // Unsupported format should still work (Other format)
        assert_eq!(
            strategy.extract_icon_name(&PathBuf::from("/path/to/app.ico")),
            Some("app".to_string())
        );
        
        // No extension
        assert_eq!(
            strategy.extract_icon_name(&PathBuf::from("/path/to/app")),
            None
        );
    }

    #[test]
    fn test_icon_detection_with_temp_directory() {
        let temp_dir = create_test_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        let mut strategy = DirectoryStrategy::with_directories(vec![icons_path]);
        strategy.initialize().unwrap();
        
        // Test finding a direct icon
        let context = IconContext::new("firefox".to_string());
        let result = strategy.detect_icon(&context);
        assert!(result.is_some());
        
        let result = result.unwrap();
        assert_eq!(result.strategy_used, "DirectoryStrategy");
        assert!(result.path.to_string_lossy().contains("firefox.png"));
        
        // Test finding an icon in subdirectory
        let context = IconContext::new("gimp".to_string());
        let result = strategy.detect_icon(&context);
        assert!(result.is_some());
        
        // Test not finding an icon
        let context = IconContext::new("nonexistent".to_string());
        let result = strategy.detect_icon(&context);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_preference() {
        let temp_dir = TempDir::new().unwrap();
        let icons_dir = temp_dir.path().join("icons");
        fs::create_dir_all(&icons_dir).unwrap();
        
        // Create same icon in different formats
        fs::write(icons_dir.join("app.png"), b"fake png").unwrap();
        fs::write(icons_dir.join("app.svg"), b"fake svg").unwrap();
        fs::write(icons_dir.join("app.xpm"), b"fake xpm").unwrap();
        
        let mut strategy = DirectoryStrategy::with_directories(vec![icons_dir]);
        strategy.initialize().unwrap();
        
        let context = IconContext::new("app".to_string());
        let result = strategy.detect_icon(&context);
        
        assert!(result.is_some());
        let result = result.unwrap();
        
        // Should prefer SVG (vector format)
        assert!(result.path.to_string_lossy().ends_with(".svg"));
        assert!(matches!(result.metadata.format, IconFormat::Svg));
    }

    #[test]
    fn test_cache_functionality() {
        let temp_dir = create_test_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        let mut strategy = DirectoryStrategy::with_directories(vec![icons_path]);
        strategy.initialize().unwrap();
        
        // First search should populate cache
        let context = IconContext::new("firefox".to_string());
        let result1 = strategy.detect_icon(&context);
        assert!(result1.is_some());
        
        // Second search should use cache
        let result2 = strategy.detect_icon(&context);
        assert!(result2.is_some());
        assert_eq!(result1.unwrap().path, result2.unwrap().path);
        
        // Check cache stats
        let (total, expired) = strategy.cache_stats().unwrap();
        assert!(total > 0);
        assert_eq!(expired, 0); // Should not be expired immediately
    }

    #[test]
    fn test_cache_expiration() {
        let temp_dir = create_test_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        let mut strategy = DirectoryStrategy::with_directories(vec![icons_path])
            .with_cache_ttl(Duration::from_millis(1)); // Very short TTL
        strategy.initialize().unwrap();
        
        // First search
        let context = IconContext::new("firefox".to_string());
        strategy.detect_icon(&context);
        
        // Wait for cache to expire
        std::thread::sleep(Duration::from_millis(10));
        
        // Check that cache entries are expired
        let (total, expired) = strategy.cache_stats().unwrap();
        assert!(total > 0);
        assert!(expired > 0);
    }

    #[test]
    fn test_max_depth_limiting() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join("icons");
        
        // Create deep directory structure
        let deep_dir = base_dir.join("level1/level2/level3/level4");
        fs::create_dir_all(&deep_dir).unwrap();
        fs::write(deep_dir.join("deep-icon.png"), b"fake png").unwrap();
        
        // Strategy with max depth 2 should not find the deep icon
        let mut strategy = DirectoryStrategy::with_directories(vec![base_dir.clone()])
            .with_max_depth(2);
        strategy.initialize().unwrap();
        
        let context = IconContext::new("deep-icon".to_string());
        let result = strategy.detect_icon(&context);
        assert!(result.is_none());
        
        // Strategy with max depth 5 should find it
        let mut strategy = DirectoryStrategy::with_directories(vec![base_dir])
            .with_max_depth(5);
        strategy.initialize().unwrap();
        
        let result = strategy.detect_icon(&context);
        assert!(result.is_some());
    }

    #[test]
    fn test_cleanup() {
        let temp_dir = create_test_icon_structure();
        let icons_path = temp_dir.path().join("icons");
        
        let mut strategy = DirectoryStrategy::with_directories(vec![icons_path]);
        strategy.initialize().unwrap();
        
        // Populate cache
        let context = IconContext::new("firefox".to_string());
        strategy.detect_icon(&context);
        
        let (total_before, _) = strategy.cache_stats().unwrap();
        assert!(total_before > 0);
        
        // Cleanup should clear cache
        strategy.cleanup();
        
        let (total_after, _) = strategy.cache_stats().unwrap();
        assert_eq!(total_after, 0);
    }
}